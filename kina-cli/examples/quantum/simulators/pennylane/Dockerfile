# Default mirrors PYTHON_BASE_IMAGE in versions.env (the pin authority);
# build-images.sh passes it explicitly.
ARG PYTHON_BASE_IMAGE=python:3.13.14-slim@sha256:6771159cd4fa5d9bba1258caf0b82e6b73458c694d178ad97c5e925c2d0e1a91
FROM ${PYTHON_BASE_IMAGE}

ARG PENNYLANE_VERSION
ARG PENNYLANE_QISKIT_VERSION

# pennylane-qiskit provides qml.from_qasm's QASM loader.
RUN pip install --no-cache-dir \
    "pennylane==${PENNYLANE_VERSION}" \
    "pennylane-qiskit==${PENNYLANE_QISKIT_VERSION}"

WORKDIR /app
COPY run_circuit.py /app/run_circuit.py

ENTRYPOINT ["python", "/app/run_circuit.py"]
