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
package org.astonbitecode.j4rs.api.value;

import org.astonbitecode.j4rs.api.JsonValue;
import org.astonbitecode.j4rs.api.ObjectValue;

public class NullJsonValueImpl implements JsonValue, ObjectValue {
    private Object obj;
    private String json;
    @SuppressWarnings("unused")
    private String className;

    NullJsonValueImpl() {
        this.obj = null;
        this.json = "null";
        this.className = NullObject.class.getName();
    }

    @Override
    public String getJson() {
        return this.json;
    }

    @Override
    public Object getObject() {
        return this.obj;
    }

    @Override
    public Class<?> getObjectClass() {
        return NullObject.class;
    }
}
