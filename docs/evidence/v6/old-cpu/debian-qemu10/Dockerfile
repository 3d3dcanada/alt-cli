FROM debian@sha256:a29215f6a35e51e22adffa17f89e9d2ef06214e64a2bad10d765c46aea49f11f
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends -y qemu-user python3 libgomp1 libnuma1 libssl3t64 libstdc++6 && rm -rf /var/lib/apt/lists/*
