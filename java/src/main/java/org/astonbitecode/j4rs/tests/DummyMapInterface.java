package org.astonbitecode.j4rs.tests;

import java.util.Map;

public interface DummyMapInterface<K, V> extends Map<K, V> {
    public long keysLength();
}