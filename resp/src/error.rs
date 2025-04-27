#[derive(Debug, PartialEq)]
pub enum Error {
    /// El payload a deserializar no tiene bytes.
    EmptyPayload,

    /// El formato de los bytes a serializar no respeta el formato establecido por RESP.
    InvalidEncoding,

    /// Falta un CRLF terminator en una posición esperada.
    MissingTerminator,

    /// La longitud del elemento bulk no puede ser deserializada como un valor entero positivo. No
    /// confundir con [`Error::WrongBulkLength`]
    InvalidBulkLength,

    /// La longitud del elemento bulk no es la esperada. No confundir con [`Error::InvalidBulkLengthEncoding`].
    ///
    /// Esto sucede en los siguientes casos:
    /// - Bulk String: la cantidad de bytes indicada no es la cantidad de bytes que componen la data de la string.
    /// - Array: la cantidad de elementos indicado no es la cantidad de elementos que componen la data del array.
    WrongBulkLength,

    /// El prefijo de los bytes a deserializar no es el esperado.
    WrongPrefix,
}
