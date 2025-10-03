# Replacing the JSON parser



`j4rs` uses (de)serialization to achieve interoperation between the Rust and Java worlds and handle custom objects (more details [here](../README.md#Passing-custom-arguments-from-Rust-to-Java)).

In the Rust world, this is achieved using [`serde`](https://github.com/serde-rs/serde) and in the Java world using [`jackson`](https://github.com/FasterXML/jackson).

For the Java world, in case `jackson` is not enough, or you just prefer using some other serialization library, `j4rs` offers the ability to replace the serializer using Java's [Service Provider Interface](https://docs.oracle.com/javase/tutorial/sound/SPI-intro.html).

To do this, you should just provide an implementation of the `Codec` interface (provided by the `j4rs` jar) and declare this implementation in a file with the name `org.astonbitecode.j4rs.api.services.json.Codec`, inside your Java library. The file should just contain the implementation class name.

You may find an example in the [j4rs-java-call-rust](https://github.com/astonbitecode/j4rs-java-call-rust) repository ([service file in META-INF](https://github.com/astonbitecode/j4rs-java-call-rust/blob/b37c9365c1b361b66ddb68084f55718a08f574a9/java/src/main/resources/META-INF/services/org.astonbitecode.j4rs.api.services.json.Codec#L1)). 

__Note__: The hash (`#`) in the start of the line is a comment and should not be there in your real implementation. Only the class name should be there.

Finally, if you want to reduce your library's size, you you may [exclude the jackson transitive dependencies of `j4rs`](https://github.com/astonbitecode/j4rs-java-call-rust/blob/b37c9365c1b361b66ddb68084f55718a08f574a9/java/pom.xml#L23). 

This was implemented in the context of issue [#62](https://github.com/astonbitecode/j4rs/issues/62)