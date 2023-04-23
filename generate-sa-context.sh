#!/bin/bash

APP_NAMESPACE=app-namespace
INGRESS_NAMESPACE=edge-services
SERVICE_ACCOUNT=fw-username
CLUSTER_NAME=cluster-name

kubectl apply -f service-account.yaml

TOKEN=$(kubectl describe secrets \
 "$(kubectl describe serviceaccount $SERVICE_ACCOUNT -n $APP_NAMESPACE \
 | grep -i Tokens | awk '{print $2}')" -n $APP_NAMESPACE | grep token: | awk '{print $2}')

kubectl config set-credentials $SERVICE_ACCOUNT --token=$TOKEN --kubeconfig config

kubectl config set-context $SERVICE_ACCOUNT \
--cluster=$CLUSTER_NAME \
--user=$SERVICE_ACCOUNT --namespace $INGRESS_NAMESPACE --kubeconfig config

kubectl config use-context $SERVICE_ACCOUNT --kubeconfig config

echo "Service account successfully created"
echo "If you want to undo this action, execute 'kubectl delete -f service-account.yaml && rm config'"
echo "Please add your cluster to config"
