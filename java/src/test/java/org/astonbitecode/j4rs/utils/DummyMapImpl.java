/*
 * Copyright 2019 astonbitecode
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
package org.astonbitecode.j4rs.utils;

import java.util.HashMap;
import java.util.stream.Collectors;

public class DummyMapImpl extends HashMap<String, Object> implements DummyMapInterface<String, Object> {
    private static final long serialVersionUID = 1L;

    public DummyMapImpl() {
        put("one", 1);
        put("two", 2);
    }

    public long keysLength() {
        return keySet().stream().map(String::length).collect(Collectors.summingInt(Integer::intValue));
    }
}