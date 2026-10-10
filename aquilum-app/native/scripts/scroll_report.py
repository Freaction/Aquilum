import json
import sys


def main(path):
    data = json.load(open(path, encoding="utf-8"))["traceEvents"]
    events = sorted((e["ts"] / 1000, e["name"], e["args"]["value"]) for e in data if e.get("ph") == "C")
    last = None
    for ts, name, value in events:
        if name == "прокрутка: смещение":
            if value == last:
                continue
            last = value
        print(f"{ts:10.1f} мс  {name:22} {value:10.2f}")


if __name__ == "__main__":
    main(sys.argv[1])
