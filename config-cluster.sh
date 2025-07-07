#!/bin/bash

spinner() {
    local pid=$1
    local msg=$2
    local spin='⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏'
    local i=0
    while kill -0 $pid 2>/dev/null; do
        i=$(( (i+1) % 10 ))
        printf "\x1b[12K\r${spin:i:1} \x1b[38;2;107;114;128m$msg\x1b[0m"
        sleep 0.1
    done
}

conectar_nodos() {
    title "Haciendo MEET entre nodos"

    base_port=$1
    shift
    other_ports=("$@")

    for port in "${other_ports[@]}"; do
        redis_cmd "redis-cli -h 172.20.0.10 -p 7000 cluster meet 172.20.0.1$port 700$port"
    done
}

asignar_slots() {
    title "Asignando hash slots"

    redis_cmd "redis-cli -h 172.20.0.10 -p 7000 cluster addslotsrange 0 5460"
    redis_cmd "redis-cli -h 172.20.0.13 -p 7003 cluster addslotsrange 5461 10921"
    redis_cmd "redis-cli -h 172.20.0.16 -p 7006 cluster addslotsrange 10922 16383"
}

asignar_replicas() {
    master_port=$1
    shift
    replicas_ports=("$@")

    master_id=$(redis-cli -h 172.20.0.1$master_port -p 700$master_port cluster myid)
    subtitle "Nodo $master_port (id: $master_id)\n"

    for port in "${replicas_ports[@]}"; do
        redis_cmd "redis-cli -h 172.20.0.1$port -p 700$port cluster replicate $master_id"
    done
}

title() {
    printf "\x1b[1m\x1b[92m$1\x1b[0m\n"
}

subtitle() {
    printf "\x1b[38;2;107;114;128m> $1\x1b[0m"
}

redis_cmd() {
    res=$(eval "$1")
    if [ "$res" = "OK" ]; then
        printf "$1 \x1b[92mOK\x1b[0m\n"
    else
        printf "$1 \x1b[38;5;208m(%s)\x1b[0m\n" "$res"
    fi
}

redis_demo() {
    echo "
       ┌──────┐              ┌──────┐              ┌──────┐       
       │ 7000 │              │ 7003 │              │ 7006 │       
       └──┬┬──┘              └──┬┬──┘              └──┬┬──┘       
 ┌──────┐ ││ ┌──────┐  ┌──────┐ ││ ┌──────┐  ┌──────┐ ││ ┌──────┐ 
 │ 7001 ◄─┘└─► 7002 │  │ 7004 ◄─┘└─► 7005 │  │ 7007 ◄─┘└─► 7008 │ 
 └──────┘    └──────┘  └──────┘    └──────┘  └──────┘    └──────┘ 
└────────────────────┘└────────────────────┘└────────────────────┘
        0-5460              5461-10921           10922-16383      
    "

    ports=($(seq 0 8))

    conectar_nodos "${ports[@]}"
    echo

    # Este `sleep` es solo para darles tiempos a los nodos a que se conozcan
    # para que cuando asignemos los master a los slaves, estos nodos ya hayan
    # conocido al nodo que va a ser su master.
    # Si no, lo que se tendría que hacer sería correr el script multiples
    # veces, o al menos la parte de asignación de los masters faltantes tras
    # cada ejecución.

    (sleep 3) &
    spinner $! "Esperando que los nodos se conozcan..."
    echo; echo

    asignar_slots
    echo

    title "Asignando replicas a masters"

    asignar_replicas 0 1 2
    echo

    asignar_replicas 3 4 5
    echo

    asignar_replicas 6 7 8
    echo

    while true; do
        (sleep 1) &
        spinner $! "Verificando estado del cluster..."
        echo

        result=$(redis-cli -h 172.20.0.16 -p 7006 get docs_ids 2>&1)
        if [[ "$result" != *"UNSET"* ]]; then
            break
        fi
        sleep 2
    done

    title "¡Cluster Redis configurado correctamente!"
}

redis_demo
