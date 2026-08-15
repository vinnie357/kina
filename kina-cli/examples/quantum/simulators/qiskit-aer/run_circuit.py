import argparse
import json
import sys

from qiskit import QuantumCircuit
from qiskit_aer import AerSimulator


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--circuit", required=True)
    parser.add_argument("--shots", type=int, default=1024)
    parser.add_argument("--output", default="/output/result.json")
    args = parser.parse_args()

    circuit = QuantumCircuit.from_qasm_file(args.circuit)
    result = AerSimulator().run(circuit, shots=args.shots).result()
    counts = result.get_counts()

    print(f"Measurement counts: {counts}")
    with open(args.output, "w", encoding="utf-8") as output:
        json.dump({"counts": dict(counts), "shots": args.shots}, output, indent=2)
    return 0


if __name__ == "__main__":
    sys.exit(main())
