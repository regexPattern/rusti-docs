
// libreria SDK con funciones que c rean threads....ver
// RESPONSABILIDADES  

// CADA THREAD RECIBE 
//     UN IP, 
//     PUERTO 
//     UN COMANDO
//     UN CHANNEL PARA COMNICARSE COIN LA GUI
// - MANEJAR LOS STREAMS Y CREARLOS Y DEVUELVE REPLIES EN RESP
// fn thread para comandos generales (){
//     PUEDE NI SER UN THREAD AQUI (SOLO PROCESA Y DISPATCHEA)
// }
// fn thread para escritura de comandos pub sub (){
// }
// fn thread para lectura de comandos pub sub() {
// }


// pub fn spawn_general_command_handler(
//     addr: &str,
//     rx: Receiver<String>, // recibe comandos de la GUI
//     tx: Sender<String>,   // envía respuestas a la GUI
// );

// pub fn spawn_pubsub_writer(
//     addr: &str,
//     rx: Receiver<String>, // recibe comandos pubsub de la GUI
//     tx: Sender<String>,   // opcional: para logs o acks
// );

// pub fn spawn_pubsub_reader(
//     addr: &str,
//     tx: Sender<String>,   // envía mensajes pubsub a la GUI
// );

