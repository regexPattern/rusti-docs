# Taller de Programación - redis-taceans

Implementación de Redis con soporte para pub/sub y Redis Cluster, un editor de texto colaborativo, un microservicio de control y persistencia que utiliza la implementación de Redis como backend, y un microservicio de generación de contenido usando LLMs.

![Rust-docs](https://private-user-images.githubusercontent.com/47466248/563577385-21291241-70b6-4f86-8b31-c2e46b3b2803.webp?jwt=eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpc3MiOiJnaXRodWIuY29tIiwiYXVkIjoicmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbSIsImtleSI6ImtleTUiLCJleHAiOjE3NzQ0Nzk1NjQsIm5iZiI6MTc3NDQ3OTI2NCwicGF0aCI6Ii80NzQ2NjI0OC81NjM1NzczODUtMjEyOTEyNDEtNzBiNi00Zjg2LThiMzEtYzJlNDZiM2IyODAzLndlYnA_WC1BbXotQWxnb3JpdGhtPUFXUzQtSE1BQy1TSEEyNTYmWC1BbXotQ3JlZGVudGlhbD1BS0lBVkNPRFlMU0E1M1BRSzRaQSUyRjIwMjYwMzI1JTJGdXMtZWFzdC0xJTJGczMlMkZhd3M0X3JlcXVlc3QmWC1BbXotRGF0ZT0yMDI2MDMyNVQyMjU0MjRaJlgtQW16LUV4cGlyZXM9MzAwJlgtQW16LVNpZ25hdHVyZT1mMzViYzM2NTM0ZTMyNWIwYTYyNTI2NWQ1NjM3M2Q5NjZhYTZjYzEyNmE5NmZiMGZjMDc0NDU0NzMzODAyYjY4JlgtQW16LVNpZ25lZEhlYWRlcnM9aG9zdCJ9.2bIeHh6gGVnkx_y7R54rkj-Vi072UhFaLqTluBCWAEM)

Proyecto realizado durante la cursada de la materia Taller de Programación FIUBA durante el 1C 2025. [Enunciado del proyecto](https://taller-1-fiuba-rust.github.io/proyecto/25C1/proyecto.html).

## Integrantes

| Nombre          | Padrón |
| --------------- | ------ |
| Douce Germán    | 106001 |
| Carlos Castillo | 108535 |
| Lucas Araujo    | 109867 |

## Cluster y Microservicios

Para reconstruir el cluster propuesto por la consigna y presentando durante la demo final se debe levantar el archivo [`compose.yaml`](./compose.yaml) utilizando Docker Compose. Antes de esto se debe configurar la variable de entorno `OPENAI_API_KEY` para la correcta inicialización del microservicio de generación de contenido. Para mayor información sobre cómo obtener esta llave de acceso podes consultar la [documentación oficial de OpenAI](https://help.openai.com/en/articles/4936850-where-do-i-find-my-openai-api-key). Un ejemplo de cómo definir esta variable es exportándola utilizando el siguiente comando con el valor de tu llave correspondiente:

```bash
export OPENAI_API_KEY=...
```

Una vez hecho esto, se debe ejecutar el siguiente comando desde la raíz del repositorio:

```bash
docker compose up -d
```

En este archivo se definen los servicios de los 9 nodos que van a formar parte del cluster, y que tienen asignados y exponen el rango de puertos del `7000` al `7008`. Este compose también define los microservicios y los conecta al cluster a través de la misma red interna de Docker.

### Configuración del Cluster

Una vez que los nodos se hayan inicializado, se debe configurar el cluster, es decir, se deben introducir algunos nodos entre sí para iniciar a compartir información entre ellos y que inicien a conocerse, y luego se deben asignar roles de master y replica entre los nodos iniciados, así como también los hash slots correspondientes a cada uno de los shards deseados.

La configuración propuesta por la consigna consta de 3 nodos master con 2 réplicas cada uno. Por practicidad, en el ejemplo provisto se designan como master a los nodos con puertos `7000`, `7003` y `7006`, y como réplicas de cada master, a los dos nodos que le siguen numéricamente en cuanto a puerto. Para la asignación del rango de hash slots, simplemente se dividió el total de slots entre 3 rangos de igual longitud, que es lo mismo que hace el cliente oficial de Redis al utilizar la utilidad de construcción de cluster en su configuración por defecto.

La estructura se puede visualizar faciltamente en el siguiente gráfico:

```text
          ┌──────┐              ┌──────┐              ┌──────┐
          │ 7000 │              │ 7003 │              │ 7006 │
          └──┬┬──┘              └──┬┬──┘              └──┬┬──┘
    ┌──────┐ ││ ┌──────┐  ┌──────┐ ││ ┌──────┐  ┌──────┐ ││ ┌──────┐
    │ 7001 ◄─┘└─► 7002 │  │ 7004 ◄─┘└─► 7005 │  │ 7007 ◄─┘└─► 7008 │
    └──────┘    └──────┘  └──────┘    └──────┘  └──────┘    └──────┘
   └────────────────────┘└────────────────────┘└────────────────────┘
           0-5460              5461-10921           10922-16383
```

La configuración del cluster se hace de manera automática al inicializar el compose utilizando el script [`config-cluster.sh`](./config-cluster.sh). Se espera a que el cluster esté bien configurado para que finalmente pueda ser utilizado.

### Detención y Destrucción del Cluster

Para detener el cluster y los microservicios debe ejecutar el siguiente comando:

```bash
docker compose stop
```

Para eliminarlos por completo se debe ejecutar el siguiente comando:

```bash
docker compose down
```

Si se deseara volver a levantar el cluster y los microservicios, simplemente se repite la secuencia de pasos especificada anteriormente.

### Métricas de Rendimiento y Logs

Cuando se levanta el Docker Compose, los servidores y microservicios se ejecutan en sus respectivos contenedores. Si queremos observar los logs que emiten todos los contenedores del compose, debemos ejecutar el siguiente comando:

```bash
docker compose logs
```

Así mismo, Docker Compose también provee una forma sencilla de observar el rendimiento de los servicios que corren en un mismo compose. Para esto se debe ejecutar el siguiente comando:

```bash
docker compose stats
```

## Editor de Documentos

Para compilar y ejecutar el microservicio de edición colaborativa se puede ejecutar el siguiente comando desde la raíz del repositorio:

```bash
REDIS_PORT=7000 cargo r --release -p docs_editor
```

El editor de documentos está configurado para, por defecto, conectarse al puerto por defecto de Redis, por eso, para ser utilizado con la configuración de cluster de ejemplo, se le debe pasar como variable de entorno el puerto de uno de los servidores a los que debe conectarse.

Para detener el mismo simplemente se puede hacer click en la opción de cerrar en la ventana de la aplicación.
