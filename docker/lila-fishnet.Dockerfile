FROM sbtscala/scala-sbt:eclipse-temurin-jammy-21.0.1_12_1.9.7_3.3.1

RUN git clone -b fix/pub-sub https://github.com/lenguyenthanh/rediculous.git && \
    cd rediculous && \
    sbt +publishLocal

WORKDIR /lila-fishnet

ENTRYPOINT \
    LOCAL_REDICULOUS_VERSION=$(ls /root/.ivy2/local/io.chrisdavenport/rediculous_3) && \
    sed -i 's/rediculous.*/rediculous" % "'$LOCAL_REDICULOUS_VERSION'"/' project/Dependencies.scala && \
    echo "REDIS_HOST=redis" >> .env && \
    sbt app/run
