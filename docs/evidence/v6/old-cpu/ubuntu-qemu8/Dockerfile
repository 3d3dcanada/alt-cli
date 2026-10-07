FROM ubuntu@sha256:534baea6a22c03a63003dbc8dbe78fe34bc0d7e595d9a9dc9834884ff530eb55
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends -y qemu-user python3 libgomp1 libnuma1 libssl3t64 && rm -rf /var/lib/apt/lists/*
