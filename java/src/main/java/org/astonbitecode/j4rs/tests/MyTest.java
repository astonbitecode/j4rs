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
package org.astonbitecode.j4rs.tests;

import java8.util.J8Arrays;
import java8.util.stream.Collectors;
import java8.util.stream.StreamSupport;
import org.astonbitecode.j4rs.api.invocation.NativeCallbackSupport;

import java.util.LinkedList;
import java.util.List;

public class MyTest extends NativeCallbackSupport {
    private String string;
    public static String StaticString = "This is a static String from Java";

    public MyTest() {
        this.string = "THE DEFAULT CONSTRUCTOR WAS CALLED";
    }

    public MyTest(MyTest myTest) {
        this.string = myTest.string;
    }

    public MyTest(String str) {
        this.string = str;
    }

    public MyTest(String... args) {
        this.string = J8Arrays.stream(args).collect(Collectors.joining(", "));
    }

    public static void useLongPrimitivesArray(long[] args) {
    }

    public String getMyString() {
        return string;
    }

    public String appendToMyString(String str) {
        this.string = this.string + str;
        return this.string;
    }

    public String getMyWithArgs(String arg) {
        return string + arg;
    }

    public String getMyWithArgsList(String... args) {
        String str = J8Arrays.stream(args)
                .reduce(
                        "",
                        (a, b) -> {
                            return a + b;
                        }
                );
        return str;
    }

    public List<Integer> getNumbersUntil(Integer until) {
        if (until == null) {
            return new ArrayList<>();
        } else {
            List<Integer> ints = new LinkedList<>();
            for (int i = 0; i < until; i++) {
                ints.add(i);
            }
            return ints;
        }
    }

    public Integer addInts(Integer... args) {
        int result = J8Arrays.stream(args)
                .reduce(
                        0,
                        (a, b) -> {
                            return a + b;
                        }
                );
        return result;
    }

    public Integer addInts(int a, int b) {
        return a + b;
    }

    public void list(List<String> l) {
        String str = StreamSupport.stream(l)
                .reduce(
                        "The arguments passed where",
                        (a, b) -> {
                            return a + "\n" + b;
                        }
                );
    }

    public void aMethod() {
        System.out.println("A METHOD CALLED");
    }

    public static void StaticMethod() {
        System.out.println("Static");
    }

    public void performCallback() {
        new Thread(() -> {
            doCallback("THIS IS FROM CALLBACK!");
        }).start();
    }

    public <T> T echo(T o) {
        return o;
    }

    public DummyMapInterface<String, Object> getMap() {
        return new DummyMapImpl();
    }

    public Integer getNullInteger() {
        return null;
    }

}
