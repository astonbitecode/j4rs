/*
 * Copyright 2023 astonbitecode
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
package org.astonbitecode.j4rs.json;

import org.astonbitecode.j4rs.api.services.json.Codec;

import java.util.Iterator;
import java.util.ServiceLoader;

public class JsonCodecService {
    private static JsonCodecService instance;
    private final Codec jsonCodec;

    static synchronized JsonCodecService getInstance() {
        if (instance == null) {
            instance = new JsonCodecService();
        }
        return instance;
    }

    private JsonCodecService() {
        Codec jsonCodec = null;
        ServiceLoader<Codec> loader = ServiceLoader.load(Codec.class);
        Iterator<Codec> jsonCodecs = loader.iterator();
        while (jsonCodec == null && jsonCodecs.hasNext()) {
            jsonCodec = jsonCodecs.next();
        }
        if (jsonCodec == null) {
            this.jsonCodec = new JacksonCodec();
        } else {
            this.jsonCodec = jsonCodec;
        }
    }

    public static Codec getJsonCodec() {
        return getInstance().jsonCodec;
    }
}
