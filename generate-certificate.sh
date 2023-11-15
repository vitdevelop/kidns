#!/bin/bash

cd "$1" || exit

openssl genrsa -out domain.key --traditional 2048
openssl req -x509 -new -key domain.key -out domain.crt -days 365 -SHA256 \
  -subj "/C=AU/ST=State/O=Organisation/CN=*.foo.bar" -extensions v3_ca
