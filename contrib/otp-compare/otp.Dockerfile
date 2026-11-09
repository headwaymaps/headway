FROM maven:3.9.11-eclipse-temurin-25 AS build
ARG OTP_COMMIT
ADD https://github.com/headwaymaps/OpenTripPlanner/archive/${OTP_COMMIT}.tar.gz /otp.tar.gz
WORKDIR /src
RUN tar xzf /otp.tar.gz --strip-components=1
RUN --mount=type=cache,target=/root/.m2,sharing=locked \
    mvn --batch-mode --quiet -Dmaven.test.skip -Dmaven.gitcommitid.skip=true \
      -P prettierSkip,checkstyleSkip --projects otp-shaded --also-make package \
    && cp "$(ls otp-shaded/target/otp-shaded-*.jar | grep -v sources)" /otp.jar

FROM eclipse-temurin:25-jre
COPY --from=build /otp.jar /otp.jar
