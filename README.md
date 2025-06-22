# Taller de Programación - redis-taceans

Implementación spec-compliant de Redis con soporte para pub/sub y Redis Cluster. Además de un editor de texto colaborativo y un microservicio de control y persistencia que utiliza la implementación de Redis como backend.

Proyecto realizado durante la cursada de la materia Taller de Programación FIUBA durante el 1C 2025. [Enunciado del proyecto](https://taller-1-fiuba-rust.github.io/proyecto/25C1/proyecto.html).

## Integrantes

| Nombre          | Padrón |
| --------------- | ------ |
| Douce Germán    | 106001 |
| Carlos Castillo | 108535 |
| Lucas Araujo    | 109867 |

## Instalación y Ejecución

Para instalar el programa se debe descargar este repositorio y compilar y ejecutar manualmente. La compilación y ejecución del programa requieren un versión de Rust igual o superior a Rust 2024 (v1.85.0) con el comando `cargo` accesible. También se quiere de la utilización de un sistema operativo basado en UNIX.

### Servidor

Para compilar y ejecutar el servidor de la base de datos, se puede correr el siguiente comando desde la raíz del repositorio:

```bash
cargo run --release --package redis_server
```

El servidor puede ser configurado mediante el archivo `redis.conf`, pasando la ruta al mismo como argumento al programa. Por ejemplo, ejecutando el siguiente comando desde la raíz del respositorio:

```bash
cargo run --release --package redis_server -- ./redis-server/redis.conf
```

Un ejemplo del archivo de configuración con las opciones soportada se encuentra en [este archivo](./redis-server/redis.conf). También se puede encontrar un ejemplo de las opciones para modo cluster en cada uno de los subdirectorios de [este directorio](redis-server/test-cluster/).

### Microservicio

Para compilar y ejecutar el microservicio de control y persistencia se puede ejecutar el siguiente comando desde la raíz del repositorio:

```bash
cargo run --release --package docs_syncer
```

Nótese que la base de datos debe estar escuchando conexiones para que el microservicio cumpla su con función.

### Editor

Para compilar y ejecutar el microservicio de control y persistencia se puede ejecutar el siguiente comando desde la raíz del repositorio:

```
cargo run --release --package docs_editor
```

Nótese que la base de datos debe estar escuchando conexiones y el microservicio debe estar activo para poder acceder y editar los documentos.
