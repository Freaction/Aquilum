import bisect
import json
import sys
from collections import Counter


def load(path):
    return json.load(open(path, encoding="utf-8"))["traceEvents"]


def counters(events, name):
    return sorted((e["ts"] / 1000, e["args"]["value"]) for e in events if e.get("ph") == "C" and e["name"] == name)


def predictor_jank(deltas):
    janky = counted = 0
    for d1, d2, d3 in zip(deltas, deltas[1:], deltas[2:]):
        d1, d2, d3 = abs(d1), abs(d2), abs(d3)
        if min(d1, d2, d3) <= 0:
            continue
        counted += 1
        threshold = 1.2 if max(d1, d2, d3) > 7 else 1.4
        if d2 / max(d1, d3) >= threshold or min(d1, d3) / d2 >= threshold:
            janky += 1
    return f"{janky} из {counted} = {100 * janky / max(counted, 1):.1f}%"


def presents(events):
    shown, frame = {}, None
    for _, name, value in sorted((e["ts"], e["name"], e["args"]["value"]) for e in events if e.get("ph") == "C" and e["name"].startswith("показ")):
        if name == "показ: кадр":
            frame = int(value)
        elif frame is not None:
            shown.setdefault(frame, int(value))
    ids = sorted(shown)
    gaps = Counter(shown[b] - shown[a] for a, b in zip(ids, ids[1:]) if b == a + 1)
    print("тактов на кадр -> число кадров:", dict(sorted(gaps.items())))


def scroll(events, start, scale):
    offsets = counters(events, "прокрутка: смещение")
    times = [t for t, _ in offsets]
    outputs = sorted(e["ts"] / 1000 + e["dur"] / 1000 for e in events if e.get("ph") == "X" and e["name"] == "вывод")
    shown, last = [], None
    for t in outputs:
        if not start <= t <= start + 1000:
            continue
        i = bisect.bisect_right(times, t) - 1
        while i >= 0 and last is not None and abs(offsets[i][1] - last) > 3000:
            i -= 1
        if i >= 0:
            last = offsets[i][1]
            shown.append(last)
    steps = [round((b - a) * scale, 1) for a, b in zip(shown, shown[1:])]
    print("шаг прокрутки по показанным кадрам, физ. px:", steps[:90])
    print("PredictorJank (Chromium):", predictor_jank([s for s in steps if s]))


def touchpad(events):
    steps = counters(events, "тачпад: пиксели")
    lag = sorted(v for _, v in counters(events, "тачпад: от такта, мс"))
    if not steps:
        return
    if lag:
        print(f"снятие DirectManipulation после такта, мс: медиана {lag[len(lag) // 2]:.2f}, p95 {lag[int(len(lag) * 0.95)]:.2f}")
    gestures, current = [], []
    for t, v in steps:
        if current and t - current[-1][0] > 100:
            gestures.append(current)
            current = []
        current.append((t, v))
    gestures.append(current)
    for n, gesture in enumerate(g for g in gestures if len(g) >= 10):
        d = [v for _, v in gesture]
        print(f"жест {n}: {len(d)} кадров, путь {sum(d):.0f} px, PredictorJank {predictor_jank(d)}")
        print("   шаги:", " ".join(f"{v:.1f}" for v in d[:60]))


if __name__ == "__main__":
    events = load(sys.argv[1])
    presents(events)
    scroll(events, float(sys.argv[2]) if len(sys.argv) > 2 else 2300.0, float(sys.argv[3]) if len(sys.argv) > 3 else 1.75)
    touchpad(events)
