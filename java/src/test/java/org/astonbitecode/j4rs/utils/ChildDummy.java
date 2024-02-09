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
package org.astonbitecode.j4rs.utils;

public class ChildDummy extends Dummy implements DummyInterface {
    public ChildDummy() {
        super();
    }

    @Override
    public void doSomething() {
        System.out.println("I am doing something...");
    }

    public int checkParam(int i) { return i; }

    public DummyMapInterface<String, Object> getMap() {
        return new DummyMapImpl();
    }

    public <T> Class<T> invokeGeneric(T check) {
        return (Class<T>) check.getClass();
    }
}
