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
import org.astonbitecode.j4rs.api.dtos.InvocationArg;
import org.astonbitecode.j4rs.api.services.json.Codec;
import org.astonbitecode.j4rs.api.services.json.exceptions.JsonCodecException;
import org.astonbitecode.j4rs.json.JsonCodecService;

public class JsonValueImpl implements JsonValue {
    private final Codec jsonCodec;
    private Object obj;
    private String json;
    @SuppressWarnings("unused")
    private String className;

    <T> JsonValueImpl(T obj) {
        this.jsonCodec = JsonCodecService.getJsonCodec();
        this.obj = obj;
        try {
            this.json = jsonCodec.encode(obj);
        } catch (JsonCodecException error) {
            throw new JsonCodecException("While creating JsonValueCallbackImpl for instance of " + obj.getClass().getName(), error);
        }
        this.className = obj.getClass().getName();
    }

    JsonValueImpl(String json, String className) {
        this.jsonCodec = JsonCodecService.getJsonCodec();
        try {
            if (className.equals(InvocationArg.CONTENTS_ARRAY)) {
                this.obj = jsonCodec.decodeArrayContents(json);
            } else {
                this.obj = jsonCodec.decode(json, className);
            }
        } catch (JsonCodecException error) {
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

    @Override
    public String getObjectClassName() {
        return this.obj.getClass().getName();
    }
}
