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

import com.fasterxml.jackson.core.JsonProcessingException;
import org.astonbitecode.j4rs.api.JsonValue;
import org.astonbitecode.j4rs.api.ObjectValue;
import org.astonbitecode.j4rs.api.dtos.InvocationArg;
import org.astonbitecode.j4rs.errors.JsonCodecException;
import org.astonbitecode.j4rs.json.Codec;

import java.io.IOException;

public class JsonValueImpl implements JsonValue, ObjectValue {
    private Codec codec = new Codec();
    private Object obj;
    private String json;
    @SuppressWarnings("unused")
    private String className;

    public JsonValueImpl(Object obj) {
        this.obj = obj;
        try {
            this.json = codec.encode(obj);
        } catch (JsonProcessingException error) {
            throw new JsonCodecException("While creating JsonValueCallbackImpl: Could not encode " + json, error);
        }
        this.className = obj.getClass().getName();
    }

    public JsonValueImpl(String json, String className) {
        try {
            if (className.equals(InvocationArg.CONTENTS_ARRAY)) {
                this.obj = codec.decodeArrayContents(json);
            } else {
                this.obj = codec.decode(json, className);
            }
        } catch (ClassNotFoundException | IOException error) {
            throw new JsonCodecException("While creating JsonValueCallbackImpl: Could not decode " + json, error);
        }
        this.json = json;
        this.className = className;
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
        return this.obj.getClass();
    }
}
