# Local Test

## create bridge network between docker-prometheus and the app

docker network create prometheus-network
docker network connect prometheus-network "name your prometheus container"
docker restart "name your prometheus container"
<!-- docker network connect prometheus-network proxy_cache_aws-prometheus-1
docker restart proxy_cache_aws-prometheus-1 -->

## in prometheus.yml put as ip: 
"host.docker.internal"