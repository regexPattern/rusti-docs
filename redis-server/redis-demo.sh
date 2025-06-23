#!/bin/bash

redis_meet() {
	base_port=$1
	shift
	other_ports=("$@")

	for port in "${other_ports[@]}"; do
		cmd="redis-cli -p $base_port cluster meet 127.0.0.1 $port"
		res=$(eval "$cmd")
		echo "$cmd => $res"
	done
}

redis_addslots() {
	cmd="redis-cli -p 7000 cluster addslotsrange 0 5460"
	res=$(eval "$cmd")
	echo "$cmd => $res"

	cmd="redis-cli -p 7003 cluster addslotsrange 5461 10921"
	res=$(eval "$cmd")
	echo "$cmd => $res"

	cmd="redis-cli -p 7006 cluster addslotsrange 10922 16383"
	res=$(eval "$cmd")
	echo "$cmd => $res"
}

redis_replicate() {
	master_port=$1
	shift
	replicas_ports=("$@")

	master_id=$(redis-cli -p "$master_port" cluster myid)
	echo "ID MASTER $master_id"

	for port in "${replicas_ports[@]}"; do
		cmd="redis-cli -p $port cluster replicate $master_id"
		res=$(eval "$cmd")
		echo "$cmd => $res"
	done
}

redis_demo() {
	echo "HACIENDO MEET ENTRE NODOS..."
	sleep 1.5
	ports=($(seq 7000 7008))
	redis_meet "${ports[@]}"

	echo
	echo "ASIGNANDO SLOTS A NODOS MASTER (7000, 7003, 7006)..."
	sleep 1.5
	redis_addslots

	echo
	echo "ASIGNANDO REPLICAS A MASTERS (7000, 7003, 7006)..."
	echo

	echo "ASIGNANDO REPLICAS A NODO 7000..."
	sleep 1.5
	redis_replicate 7000 7001 7002

	echo
	echo "ASIGNANDO REPLICAS A NODO 7003..."
	sleep 1.5
	redis_replicate 7003 7004 7005

	echo
	echo "ASIGNANDO REPLICAS A NODO 7006..."
	sleep 1.5
	redis_replicate 7006 7007 7008
}

redis_demo
