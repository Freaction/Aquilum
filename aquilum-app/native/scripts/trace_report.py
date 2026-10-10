import json
import sys
from collections import defaultdict

REFRESH_MS = 1000 / 120


def pct(values, q):
    if not values:
        return 0.0
    values = sorted(values)
    return values[min(len(values) - 1, int(len(values) * q))]


def main(path, skip_ms=300.0):
    data = json.load(open(path, encoding="utf-8"))["traceEvents"]
    threads = {e["tid"]: e["args"]["name"] for e in data if e.get("name") == "thread_name"}
    zones = [e for e in data if e.get("ph") == "X" and e["ts"] / 1000 >= skip_ms]
    marks = [e for e in data if e.get("ph") == "i" and e["ts"] / 1000 >= skip_ms]

    by_name = defaultdict(list)
    for z in zones:
        by_name[(threads.get(z["tid"], z["tid"]) if z["name"].startswith("vello: fine") or z["name"].startswith("vello: полосы") else "", z["name"])].append(z["dur"] / 1000)
    print(f"{'зона':42} {'n':>5} {'сред':>7} {'p95':>7} {'макс':>7} {'всего':>8}")
    merged = defaultdict(list)
    for (thread, name), v in by_name.items():
        merged[name].extend(v)
    for name, v in sorted(merged.items(), key=lambda kv: -sum(kv[1])):
        print(f"{name:42} {len(v):5} {sum(v)/len(v):7.2f} {pct(v, .95):7.2f} {max(v):7.2f} {sum(v):8.0f}")

    def intervals(ts):
        return [(b - a) / 1000 for a, b in zip(ts, ts[1:])]

    vblanks = sorted(m["ts"] for m in marks if m["name"] == "такт экрана")
    frames = sorted((z["ts"], z["dur"]) for z in zones if z["name"] == "кадр")
    for label, iv in [("такты экрана", intervals(vblanks)), ("начала кадров", intervals([f[0] for f in frames]))]:
        if iv:
            late = sum(1 for i in iv if i > REFRESH_MS * 1.5)
            print(f"{label}: n {len(iv)} сред {sum(iv)/len(iv):.2f} p95 {pct(iv, .95):.2f} макс {max(iv):.2f} пропусков такта {late}")

    over = [f for f in frames if f[1] / 1000 > REFRESH_MS]
    print(f"кадров дольше {REFRESH_MS:.1f} мс: {len(over)} из {len(frames)}")
    main_tid = next((z["tid"] for z in zones if z["name"] == "кадр"), None)
    for start, dur in over[:8]:
        inner = [z for z in zones if z["tid"] == main_tid and start <= z["ts"] < start + dur and z["name"] != "кадр"]
        parts = ", ".join(f"{z['name']} {z['dur']/1000:.1f}" for z in sorted(inner, key=lambda z: z["ts"]))
        print(f"  {start/1000:8.1f} мс: {dur/1000:.1f} — {parts}")


if __name__ == "__main__":
    main(sys.argv[1], float(sys.argv[2]) if len(sys.argv) > 2 else 300.0)
