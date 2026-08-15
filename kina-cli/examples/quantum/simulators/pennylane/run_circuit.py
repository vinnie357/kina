import argparse
import json
import sys

import pennylane as qml


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--circuit", required=True)
    parser.add_argument("--shots", type=int, default=1024)
    parser.add_argument("--output", default="/output/result.json")
    args = parser.parse_args()

    with open(args.circuit, encoding="utf-8") as handle:
        loaded = qml.from_qasm(handle.read(), measurements=None)

    device = qml.device("default.qubit", wires=2)

    # Shots belong on the QNode via set_shots, not the device: passing
    # shots= to qml.device is deprecated as of PennyLane 0.45.
    @qml.set_shots(args.shots)
    @qml.qnode(device)
    def circuit():
        loaded()
        return qml.counts()

    # str(key): qml.counts() returns numpy string keys, which json.dump writes
    # as plain strings but print() renders as np.str_('00').
    counts = {str(key): int(value) for key, value in circuit().items()}
    print(f"Measurement counts: {counts}")
    with open(args.output, "w", encoding="utf-8") as output:
        json.dump({"counts": counts, "shots": args.shots}, output, indent=2)
    return 0


if __name__ == "__main__":
    sys.exit(main())
