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
#### Kidns is configured through `config.yaml` file:

```yaml
dns:
  server:
    # by default will be '8.8.8.8' if not set
    public: 8.8.8.8
    # by default will be port 53 if not set
    port: 53
    # if ignored "empty" value is set, if "empty" dns disabled
    host: 0.0.0.0
    # can be set as 'k8s' to load kubernetes ingress url
    # also can load urls from file('filename')
  cache:
    - k8s
#    - local_cache.conf
k8s:
  # default(look to ~/.kube/config) or path to yaml file
  # if not set or empty by default is set 'default'
  # if set file in bin directory no need full path
  - config: config
    pod:
      # namespace where is located nginx pods
      namespace: edge-services
      label: app.kubernetes.io/name=ingress-nginx
      # if ignored, http:80 and https:443 will be set
      port:
        # default 80 if not set
        http: 80
        # default 443 if not set
        https: 443
    # namespace where need to load ingress urls, ex. your app
    ingress-namespace: app-namespace
proxy:
  # if empty, proxy disabled
  host: 0.0.0.0
  # if ignored, http:80 and https:443 will be set
  port:
    # default 80 if not set
    http: 80
    # default 443 if not set
    https: 443
  tls:
  # if not set, local tls disabled
    cert: testspace/domain.crt
    key: testspace/domain.key
log-level: info
```
###### NOTICE:
1) If you want to use dns, add value of `dns.server.host` to your OS DNS configuration.
2) If you want to use default dns port `53`, need to run app with admin privilegies.
3) If you want to use proxy port `80` or `443`, need to run app with admin privilegies.

#### If needed to generate kubernetes service-account:
1) Edit `generate-sa-context.sh` file and replace `APP_NAMESPACE, INGRESS_NAMESPACE, SERVICE_ACCOUNT, CLUSTER_NAME` with your.
2) Edit `service-account.yaml` file and replace with your config.

###### It will generate `config` file without cluster. NOTICE: Add your cluster to it.

#### If needed to run through `docker compose`
1) Edit `docker-compose.yaml` file, volumes section:
```yaml
volumes:
  - "<path-to-config(kubernetes config yaml file)>:/kidns/config:ro"
  - "<path-to-config.yaml>:/kidns/config.yaml:ro"
```
2) Run `docker compose up`