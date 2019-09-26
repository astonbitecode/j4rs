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

import org.astonbitecode.j4rs.api.invocation.NativeCallbackSupport;

import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;
import java.util.stream.IntStream;

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
        this.string = Arrays.stream(args).collect(Collectors.joining(", "));
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
        String str = Arrays.stream(args)
                .reduce(
                        "",
                        (a, b) -> {
                            return a + b;
                        }
                );
        return str;
    }

    public List<Integer> getNumbersUntil(Integer until) {
        return IntStream.range(0, until).boxed().collect(Collectors.toList());
    }

    public Integer addInts(Integer... args) {
        int result = Arrays.stream(args)
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
        String str = l.stream()
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

}
