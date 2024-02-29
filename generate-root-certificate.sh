#!/bin/bash

CANAME=MyOrg-RootCA
DOMAIN="MyOrg"

# --------------------------RSA------------------------------------------
openssl genrsa -out $CANAME.key 3072 || \
(echo "Unable to generate private key"; exit)

# create certificate, 1826 days = 5 years
openssl req -x509 -key $CANAME.key -out $CANAME.pem -days 1826 \
-extensions v3_ca \
-subj "/C=AU/ST=State-Root/O=Organisation-Root/CN=$DOMAIN" || \
(echo "Unable to create public key"; exit)

# Sign new cert and verify
#-----------------------------------
#openssl genrsa -out client.key 3072
#openssl req -new -sha256 -key client.key -out client.csr \
#-subj "/C=AU/ST=State/O=Organisation/CN=$DOMAIN"
#openssl x509 -req -in client.csr -CA $CANAME.pem -CAkey $CANAME.key -out client.pem -days 365 -sha256
#openssl verify -verbose -CAfile $CANAME.pem client.pem

# --------------------------ECDSA----------------------------------------
#openssl genpkey -algorithm Ed25519 -out $CANAME.key || \
#(echo "Unable to generate private key"; exit)
#
## create certificate, 1826 days = 5 years
#openssl req -x509 -sha384 -key $CANAME.key -out $CANAME.pem -days 1826 \
#-extensions v3_ca \
#-subj "/C=AU/ST=State-Root/O=Organisation-Root/CN=$DOMAIN" || \
#(echo "Unable to create public key"; exit)

# Sign new cert and verify
#-----------------------------------
#openssl ecparam -out client.key -name prime256v1 -genkey
#openssl req -new -sha256 -key client.key -out client.csr \
#-subj "/C=AU/ST=State/O=Organisation/CN=$DOMAIN"
#openssl x509 -req -in client.csr -CA $CANAME.pem -CAkey $CANAME.key -out client.pem -days 365 -sha256
#openssl verify -verbose -CAfile $CANAME.pem client.pem