#!/bin/bash
# Docker 测试用启动脚本（配合 --network host 使用）
# 容器共享宿主机网络，openclaw 监听 127.0.0.1:17789
# 宿主机直接访问 localhost:17789 即可，无需改任何配置

echo "[docker-start] starting openclaw gateway (network=host, port=17789)"
exec openclaw gateway --port 17789
