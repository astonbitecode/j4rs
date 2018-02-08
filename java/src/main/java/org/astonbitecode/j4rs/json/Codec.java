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
package org.astonbitecode.j4rs.json;

import com.fasterxml.jackson.core.JsonParseException;
import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.JsonMappingException;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.astonbitecode.j4rs.errors.JsonCodecException;

import java.io.IOException;
import java.lang.reflect.Array;
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;

public class Codec {
    private static final String RUST_FIELD = "Rust";
    private static final String JSON_FIELD = "json";
    private static final String CLASS_NAME_FIELD = "class_name";
    private ObjectMapper mapper = new ObjectMapper();
    TypeReference<Map<String, Object>[]> typeRef
            = new TypeReference<Map<String, Object>[]>() {
    };

    @SuppressWarnings("unchecked")
    public <T> T decode(String json, String className) throws ClassNotFoundException, IOException {
        Class<T> clazz = (Class<T>) Class.forName(className);
        T obj = mapper.readValue(json, clazz);
        return obj;
    }

    public String encode(Object obj) throws JsonProcessingException {
        return mapper.writeValueAsString(obj);
    }

    public Object[] decodeArrayContents(String json) throws IOException {
        Map<String, Object>[] array = mapper.readValue(json, typeRef);

        return Arrays.stream(array)
                .map(elem -> {
                    try {
                        return retrieveFromMap(elem);
                    } catch (Exception error) {
                        throw new JsonCodecException("Error while retrieving Array", error);
                    }
                }).toArray();
    }

    /**
     * [
     * {"Rust":{"json":"\"arg1\"","class_name":"java.lang.String","arg_from":"rust"}},
     * {"Rust":{"json":"\"arg2\"","class_name":"java.lang.String","arg_from":"rust"}},
     * {"Rust":{"json":"\"arg3\"","class_name":"java.lang.String","arg_from":"rust"}}
     * ]
     */
    private <U> U retrieveFromMap(Map<String, Object> map) throws ClassNotFoundException, IOException {
        Map<String, String> innerMap = (Map<String, String>) map.get(RUST_FIELD);
        if (innerMap == null) {
            throw new JsonCodecException("Cannot create InvocationArg object form Map '" + map + "'");
        }
        String retrievedClassName = innerMap.get(CLASS_NAME_FIELD);
        String retrievedJson = innerMap.get(JSON_FIELD);
        if (retrievedClassName == null || retrievedJson == null) {
            throw new JsonCodecException("Cannot create InvocationArg object form the JSON '" + retrievedJson + "'");
        }
        return decode(retrievedJson, retrievedClassName);
    }
}
