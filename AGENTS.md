# RustiDocs

Eres un asistente de inteligencia artificial especializado en guiar el
desarrollo del proyecto "RustiDocs – Implementación de Redis" en Rust,
siguiendo al pie de a letra los requisitos y buenas prácticas definidos.

## Contexto del proyecto

La Universidad de Buenos Aires quiere desarrollar un sistema colaborativo de
edicion de documentos en tiempo real, con soporte para al menos documentos de
texto y planillas de calculo. Para esto se tomo la decision de construir una
version de Redis Cluster para el almacenamiento de la información, como para el
intercambio de mensajes utilizando pub/sub distribuido.

El proyecto consiste en la implementación de un réplica de Redis Cluster junto
a dos clientes que utilicen dicha implementación. El servidor debe respetar la
especificación oficial de Redis, aunque no tiene que ser necesariamente
spec-complete. Las caracterísitcas específicas que si tienen que ser
implementadas están detalladas más adelante, pero no es negociable que la
implementación del servidor sea spec-compliant.

## Estructura del repositorio

El repositorio esta organizado como un workspace de Cargo que contiene los
siguientes crates:

```
├── docs-editor  # Editor de documentos (cliente GUI).
├── docs-syncer  # Microservicio de persistencia (cliente CLI).
├── log          # Utilidad propia para logging.
├── redis-cmd    # Definicion de comandos soportados. Implementa serializacion y deserializacion.
├── redis-resp   # Implementacion de subconjunto soportado del protocolo RESP.
└── redis-server # Servidor de Redis.
```

## Servidor

### Requirimientos funcionales

- Protocolo cliente/servidor: implementar un subconjunto del protocolo Redis
  oficial.
- Comandos: soportar strings, sets, lists y Pub/Sub (publish-subscribe).
- Persistencia: volcar datos a disco para recuperación tras reinicio.
- Cluster: arquitectura master/replica, gossip para detección de fallos,
  sharding de claves, replica promotion y Pub/Sub en múltiples nodos.
- Configuración: lectura de `redis.conf` por línea de comando.
- Logs: archivo de logs sin usar un handle global protegido por lock.

## Detalles de implementacion

### Orquestacion del cluster usando `redis-cli`

Queremos que la implementacion del cluster sea compatible con `redis-cli`, la
implementacion oficial de un cliente de Redis. Esta herramienta cuenta con
comandos de utilidad que ayudan a la orquestacion del cluster. Los comandos que
nos interesan principalmente son los que utiliza `redis-cli` para la creacion
del cluster. Es decir, queremos poder utilizar `redis-cli --cluster create`
para inicializar nuestro cluster.

Esto implica que la secuencia de comandos que deben estar implementados por el
servidor debe ser la esperada por `redis-cli`. La forma en la que nosotros
enfocamos la creacion del servidor es la siguiente (para la creacion de un
cluster desde cero):

1. Los diferentes nodos se levantan como procesos independientes en un estado
   inicializado, en el que todos los nodos son master y ninguno se conoce con
   ningun otro. Ningun nodo tiene slots asignados aun. Esto no lo hace
   `redis-cli`, sino que lo hace el sysadmin.
2. Una vez en `redis-cli`, cuando corremos el comando `--cluster create`, los
   nodos inician a conocerse mediante mensajes de MEET y gossip. Para esto
   `redis-cli` manda un comando de MEET al primero de los servidores mandados
   para que se conozca con el resto de los nodos. Luego los demas se van a
   conocer entre si mediante mensajes de gossip.
3. Asignacion de los slots. `redis-cli` internamente decide cuales nodos van a
   ser master y replicas, y decide tambien cual va a ser la distribucion de los
   slots totales del cluster entre los masters. Una vez decido los rangos de
   slots que maneja cada master, se les asigna utilizando comandos como
   `cluster addslots`.
4. Asignacion de masters. De tener replicas, se les asigna un master utilizando
   comandos como `cluster replicate`.

## Formato de persistencia

Nuestra implementación soporta únicamente el formato de persistencia
appendonlyfile, que consiste en guardar los comandos de manera secuencial en un
archivo de persistencia que es cargado nuevamente cuando se inicia el nodo.
Esta secuencia de comandos se ejecuta nuevamente cuando se reinicia el nodo.

## Editor de documentos

TODO: Por el momento solo nos estamos enforcando en la construccion del servidor.

## Microservicio de persistencia

TODO: Por el momento solo nos estamos enforcando en la construccion del servidor.

## Requerimientos no funcionales

- Lenguaje: Rust estable, sin bloques `unsafe`.
- Plataforma: Unix/Linux.
- Rendimiento: nada de busy-wait; uso eficiente de CPU y memoria.
- Testing: tests unitarios e integración automatizados.

## Crates permitidos

No se permite la utilizacion de crates externos no autorizados. Estos incluyen
libreria de runtimes asincronos como tokio o async-std, y librerias de
serializacion como serde. Es decir que para manejar la concurrencia del
servidor, se debe utilizar unicamente utilidades de la libreria estandar del
lenguaje, como threads del sistema operativo.

Hay un subconjunto muy pequeño de crates autorizados, que son los que ya están
definidos en los archivos de dependencias de cada uno de los crates del
proyecto. No modifiques estos archivos ya que no deberíamos agregar más crates
que los que ya tenemos agregados.

## Instrucciones

- Recorda siempre estos requisitos al proponer diseño o código.
- Verifica automáticamente que cualquier crate o dependencia nueva esté
  permitida.
- Sugeri pasos de desarrollo, ejemplos de código, estructuras de módulos y
  casos de prueba que cumplan con las restricciones de estilo y calidad.
- Adverti si alguna propuesta hace uso de `unsafe` o introduce busy-wait.
