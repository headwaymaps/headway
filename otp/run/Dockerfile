FROM debian:bullseye-slim

ENV TZ="America/New_York"

RUN apt-get update \
  && apt-get install -y --no-install-recommends curl openjdk-11-jre-headless sudo

ARG OTP_VERISON=2.1.0
ARG OTP_HASH=b4c986b1c726c7d81d255fa183d32576122ba4e50290d53e4bb40be051971134

WORKDIR /otp

RUN curl -L -o ./otp-shaded.jar "https://github.com/opentripplanner/OpenTripPlanner/releases/download/v${OTP_VERISON}/otp-${OTP_VERISON}-shaded.jar"

RUN bash -c 'echo "${OTP_HASH}  otp-shaded.jar" | sha256sum --check'

COPY rename_and_run.sh /otp

CMD [ "/otp/rename_and_run.sh" ]