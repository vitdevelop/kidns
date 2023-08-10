#!/bin/bash

function start_kidns() {
  docker-compose up -d
  sudo sed -i '1s/^/nameserver 127.0.0.1\n/' /etc/resolv.conf
}

function stop_kidns() {
  sudo sed -i '/127.0.0.1/d' /etc/resolv.conf
  docker-compose stop
}

function print_help() {
    echo "kidns.sh [ARG]
    kidns.sh start              -- Start server
    kidns.sh stop               -- Stop server
    kidns.sh help | --help | -h -- Print this message"
}

if [ $# -eq 0 ]; then
    >&2 print_help
    exit 1
fi

for i in "$@"; do
  case $i in
    start)
      start_kidns
      ;;
    stop)
      stop_kidns
      ;;
    help | -h | --help)
      print_help
      ;;
    *)
      print_help
      ;;
  esac
done
