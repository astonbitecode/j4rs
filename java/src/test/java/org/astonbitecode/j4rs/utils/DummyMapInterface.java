package org.astonbitecode.j4rs.utils;

import java.util.Map;

public interface DummyMapInterface<K, V> extends Map<K, V> {
  public long keysLength();
}