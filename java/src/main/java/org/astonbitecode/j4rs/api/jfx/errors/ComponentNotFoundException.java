/*
 * Copyright 2020 astonbitecode
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
package org.astonbitecode.j4rs.api.jfx.errors;

/**
 * Exception thrown if a JavaFX component is not found
 */
public class ComponentNotFoundException extends Exception {
    /**
     * Instantiate with a message
     * @param message A message
     */
    public ComponentNotFoundException(String message) {
        super(message);
    }

    /**
     * Instantiate with a {@link Throwable}
     * @param throwable The Throwable
     */
    @SuppressWarnings("unused")
    public ComponentNotFoundException(Throwable throwable) {
        super(throwable);
    }

    /**
     * Instantiate with a message and a {@link Throwable}
     * @param message The message
     * @param throwable The throwable
     */
    @SuppressWarnings("unused")
    public ComponentNotFoundException(String message, Throwable throwable) {
        super(message, throwable);
    }
}
