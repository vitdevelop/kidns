## KIDNS - Kubernetes Ingress DNS
[![Rust](https://github.com/vitdevelop/kidns/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/vitdevelop/kidns/actions/workflows/rust.yml)

#### Kidns serve as local dns server with ingress urls from kubernetes and proxy traffic to ingress pods(nginx).

###### This utility is needed only in case if you don't have external access to cluster.
###### Be aware, it can load your kubernetes API server if abused.

---
### Build:

1) Need installed Rust
2) Run `cargo build --release`
3) Extract binary from `target/release/kidns` to your path

### Usage:
#### Kidns is configured through `config.env` file:

```
# by default will be '8.8.8.8' if not set
DNS_SERVER_PUBLIC=8.8.8.8
DNS_SERVER_PORT=2053
# can be set as 'k8s' to load kubernetes ingress url
# also can load urls from file('filename')
# or can be used both, separated by comma(',')
DNS_CACHE=k8s,local_cache.conf
# if empty, dns disabled
DNS_SERVER_HOST=0.0.0.0
# default(look to ~/.kube/config) or path to yaml file
# if not set or empty by default is set 'default'
# if set file in bin directory no need full path
K8S_CONFIG=config
# namespace where is located nginx pods
K8S_POD_NAMESPACE=edge-services
K8S_POD_LABEL=app.kubernetes.io/name=ingress-nginx
K8S_POD_PORT=443
# namespace where need to load ingress urls, ex. your app
K8S_INGRESS_NAMESPACE=app-namespace
# if empty, proxy disabled
PROXY_HOST=0.0.0.0
# if need to access https url, port must be 443,
# otherwise you'll get error that unable to access https through http
PROXY_PORT=8443
# if not set, local tls disabled
#PROXY_TLS_CERT=testspace/server.crt
#PROXY_TLS_KEY=testspace/server.key
LOG_LEVEL=info
```
###### NOTICE:
1) If you want to use dns, add value of `DNS_SERVER_HOST` to your OS DNS configuration.
2) If you want to use default dns port `53`, need to run app with admin privilegies.
3) If you want to use proxy port `80` or `443`, need to run app with admin privilegies.

#### If need to generate kubernetes service-account:
1) Edit `generate-sa-context.sh` file and replace `APP_NAMESPACE, INGRESS_NAMESPACE, SERVICE_ACCOUNT, CLUSTER_NAME` with your.
2) Edit `service-account.yaml` file and replace with your config.

###### It will generate `config` file without cluster. NOTICE: Add your cluster to it.
