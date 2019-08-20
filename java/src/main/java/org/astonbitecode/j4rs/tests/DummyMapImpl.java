package org.astonbitecode.j4rs.tests;

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
