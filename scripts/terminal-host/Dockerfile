FROM python:3.11-bookworm@sha256:3fd8382c50bae84184460a6e1a8ff4eeac0e3ca6a560691cb56bd84d2b6ec9ca
RUN sed -i s,http://deb.debian.org,https://deb.debian.org,g /etc/apt/sources.list.d/debian.sources && apt-get update && apt-get install -y --no-install-recommends openssh-server tmux bubblewrap && rm -rf /var/lib/apt/lists/*
