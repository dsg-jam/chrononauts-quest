FROM mcr.microsoft.com/devcontainers/base:bookworm

# ENV Variables required by Jekyll
ENV LANG=en_US.UTF-8 \
    LANGUAGE=en_US:en \
    TZ=Etc/UTC \
    LC_ALL=en_US.UTF-8 \
    LANG=en_US.UTF-8 \
    LANGUAGE=en_US

COPY install.sh /install.sh
RUN bash /install.sh
