import argparse
import collections
import json
import sys

import qsimcirq
from cirq.contrib.qasm_import import circuit_from_qasm


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--circuit", required=True)
    parser.add_argument("--shots", type=int, default=1024)
    parser.add_argument("--output", default="/output/result.json")
    args = parser.parse_args()

    with open(args.circuit, encoding="utf-8") as handle:
        circuit = circuit_from_qasm(handle.read())

    result = qsimcirq.QSimSimulator().run(circuit, repetitions=args.shots)

    # Combine all measurement keys into one bitstring per shot (QASM cregs
    # import as one key per register; sort keys for determinism).
    keys = sorted(result.measurements)
    counts: collections.Counter[str] = collections.Counter()
    for shot in range(args.shots):
        bits = "".join(
            "".join(str(bit) for bit in result.measurements[key][shot])
            for key in keys
        )
        counts[bits] += 1

    print(f"Measurement counts: {dict(counts)}")
    with open(args.output, "w", encoding="utf-8") as output:
        json.dump({"counts": dict(counts), "shots": args.shots}, output, indent=2)
    return 0


if __name__ == "__main__":
    sys.exit(main())
