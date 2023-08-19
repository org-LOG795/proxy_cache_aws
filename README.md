


#Metrics

## To Start Prometheus and Grafana

docker compose -f "docker-compose.yml" up -d --build

## To access Prometheus

Go to : http://127.0.0.1:9090/

## To access Grafana

Go to : http://127.0.0.1:3000/
user: admin
password : secret 
"make sure to hide and change the password"

## In Grafana

In settings -> Data Source
Name : Prometheus
HTTP ->  put this URL: http://prometheus:9090

## Import grafana_graph.json in grafana to see the dashboard

currently there is 4 view in the dashboard for the compress and decompress 

## Add Metrics

to have more metrics on the other facades, don't hesitate to take inspiration from the lazy_static variable! create in compression.rs file


