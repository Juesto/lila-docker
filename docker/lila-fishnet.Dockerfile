FROM sbtscala/scala-sbt:eclipse-temurin-jammy-21.0.1_12_1.9.7_3.3.1

RUN git clone -b fix/pub-sub https://github.com/lenguyenthanh/rediculous.git && \
    cd rediculous && \
    sbt +publishLocal

WORKDIR /lila-fishnet

ENTRYPOINT git checkout . && \
    LOCAL_VERSION=$(ls /root/.ivy2/local/io.chrisdavenport/rediculous_3) && \
    sed -i 's/rediculous.*/rediculous" % "'$LOCAL_VERSION'"/' project/Dependencies.scala && \
    echo "REDIS_HOST=redis" > .env && \
    echo "REDIS_PORT=6379" >> .env && \
    echo "HTTP_SERVER_HOST=0.0.0.0" >> .env && \
    echo "HTTP_SERVER_PORT=9665" >> .env && \
    sbt app/run
