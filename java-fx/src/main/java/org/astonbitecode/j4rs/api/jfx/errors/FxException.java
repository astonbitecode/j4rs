package org.astonbitecode.j4rs.api.jfx.errors;

public class FxException extends Exception {
    public FxException(String message) {
        super(message);
    }

    public FxException(Throwable throwable) {
        super(throwable);
    }

    public FxException(String message, Throwable throwable) {
        super(message, throwable);
    }
}
