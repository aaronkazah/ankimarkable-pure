FROM public.ecr.aws/docker/library/ubuntu:24.04@sha256:534baea6a22c03a63003dbc8dbe78fe34bc0d7e595d9a9dc9834884ff530eb55
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates python3 make gcc g++ file xz-utils && rm -rf /var/lib/apt/lists/*
COPY --from=sdk /SDK.sh /tmp/SDK.sh
RUN bash /tmp/SDK.sh -y -d /opt/codex && rm /tmp/SDK.sh
WORKDIR /source
