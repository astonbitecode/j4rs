/*
 * Copyright 2018 astonbitecode
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
package org.astonbitecode.j4rs.api;

/**
 * Represents an object for j4rs needs
 */
public interface ObjectValue {
    /**
     * Returns the Object
     * @return An object instance
     */
    Object getObject();

    /**
     * Returns the {@link Class} of the Object
     * @return A {@link Class}
     */
    Class<?> getObjectClass();

    /**
     * Returns the name of the Object Class
     * @return A String representing the name of the Object Class
     */
    String getObjectClassName();
}
