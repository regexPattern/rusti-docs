# Taller de Programación - redis-taceans

Implementación de Redis con soporte para pub/sub y Redis Cluster, un editor de texto colaborativo, un microservicio de control y persistencia que utiliza la implementación de Redis como backend, y un microservicio de generación de contenido usando LLMs.

Proyecto realizado durante la cursada de la materia Taller de Programación FIUBA durante el 1C 2025. [Enunciado del proyecto](https://taller-1-fiuba-rust.github.io/proyecto/25C1/proyecto.html).

## Integrantes

| Nombre          | Padrón |
| --------------- | ------ |
| Douce Germán    | 106001 |
| Carlos Castillo | 108535 |
| Lucas Araujo    | 109867 |

## Instalación y Ejecución

Para reproducir la estructura del sistema empleada en las demos se tienen que ejecutar los siguientes pasos:

1. Inicialización del cluster.
2. Inicialización de los microservicios.
3. Inicialización del editor de documentos.

Las siguientes secciones detallan cómo completar cada uno de estos pasos.

### Cluster

En el archivo [`compose.yaml`](./compose.yaml) está definida una red de 9 nodos de Redis, configurados en modo cluster, escuchando en los puertos del 7000 al 7008. Para levantar los servidores de los 9 nodos se puede ejecutar el siguiente comando desde la raíz del repositorio:

```bash
docker compose up
```

Luego de esto, se procede con la configuración del cluster. Para agilizar esto durante la demo final, se escribió un script de bash que utiliza `redis-cli` para correr los comandos necesarios para recrear la estructura establecida por la consigna. Para ejecutar el script, se puede ejecutar el siguiente comando desde la raíz del repositorio:

```bash
bash ./config-cluster.sh
```

Al finalizar la configuración del cluster, la base de datos estará lista para ser utilizada por cualquier cliente de Redis.

### Microservicios

Los microservicios son clientes de Redis que se conectan a la base de datos. Por defecto, se conectan al puerto por defecto de Redis (6379), pero pueden ser configurados para conectarse con un servidor en otro puerto.

Si se utiliza el cluster construido para la demo, detallado en la sección anterior, es necesario cambiar el puerto de acceso inicial de los microservicios al momento de ejecutarlos. Esto se hace configurando la variable de entorno `REDIS_PORT` al valor del puerto deseado.

#### Microservicio de Control y Persistencia

Para compilar y ejecutar el microservicio de control y persistencia se puede ejecutar el siguiente comando desde la raíz del repositorio:

```bash
REDIS_PORT=7000 cargo run --release --package docs_syncer
```

Esto inicializa el microservicio que inicia a atender solicitudes de los editores clientes.

#### Microservicio de Generación

El microservicio de generación de contenido requiere de la configuración adicional de una variable de entorno `OPENAI_API_KEY`, cuyo valor es la clave de acceso a la API de OpenAI. Para mayor información sobre cómo obtener esta llave de acceso podes consultar la [documentación oficial de OpenAI](https://help.openai.com/en/articles/4936850-where-do-i-find-my-openai-api-key).

Ahora, de manera similar al microservicio de persistencia, para el microservicio de generación de contenido se puede ejecutar el siguiente comando:

```bash
# Reemplazamos los ... el valor de nuestra llave de acceso.
OPENAI_API_KEY=... REDIS_PORT=7000 cargo run --release --package docs_gpt
```

Esto inicializa el microservicio que inicia a atender solicitudes de los editores clientes.

### Editor

Para compilar y ejecutar el microservicio de edición colaborativa se puede ejecutar el siguiente comando desde la raíz del repositorio:

```bash
REDIS_PORT=7000 cargo run --release --package docs_editor
```

Al igual que los microservicios, el editor de documentos está configurado para, por defecto, conectarse al puerto por defecto de Redis, por eso, para ser utilizado con la configuración de cluster de ejemplo, se le debe pasar como variable de entorno el puerto de uno de los servidores a los que debe conectarse.
