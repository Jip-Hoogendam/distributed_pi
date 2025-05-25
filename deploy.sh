#!/bin/bash

echo building the docker images

docker buildx build ./hub --tag gitea.glad0s.servers/jip/pi_calc_hub:latest --platform=linux/arm64 --builder=k8s_builder --push 

docker buildx build ./spoke --tag gitea.glad0s.servers/jip/pi_calc_spoke:latest --platform=linux/arm64 --builder=k8s_builder --push

docker buildx build ./html --tag gitea.glad0s.servers/jip/pi_calc_frontend:latest --platform=linux/arm64 --builder=k8s_builder --push

echo running the k8s image

kubectl apply -f ./k8s_deployment.yaml