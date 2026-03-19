#!/bin/bash
# 启动 openclaw rootfs 测试容器
# 用法：bash docker-run-test.sh [镜像名]
# 默认镜像：openclaw-rootfs:test

IMAGE=${1:-openclaw-rootfs:test}
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "[docker-run-test] 停止旧容器..."
docker rm -f openclaw-test 2>/dev/null

echo "[docker-run-test] 启动容器（镜像: $IMAGE）..."
docker run -d \
  --name openclaw-test \
  --network host \
  -v "$SCRIPT_DIR/docker-start.sh:/docker-start.sh:ro" \
  "$IMAGE" \
  bash -lc "bash /docker-start.sh"

echo "[docker-run-test] 等待启动..."
sleep 5
docker logs openclaw-test 2>&1 | tail -10
echo ""
echo "[docker-run-test] 完成，访问 http://localhost:17789/"
