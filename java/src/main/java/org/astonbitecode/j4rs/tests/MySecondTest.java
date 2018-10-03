package org.astonbitecode.j4rs.tests;

import org.astonbitecode.j4rs.api.invocation.NativeCallbackToRustChannelSupport;

import java.util.stream.IntStream;

public class MySecondTest extends NativeCallbackToRustChannelSupport {
    public void performCallback() {
        new Thread(() -> {
            doCallback("THIS IS FROM CALLBACK TO A CHANNEL...");
        }).start();
    }

    public void performTenCallbacks() {
        new Thread(() -> {
            IntStream.range(0, 10).forEach(i -> doCallback("THIS IS FROM CALLBACK TO A CHANNEL..." + i));
        }).start();
    }

    public void performCallbackFromTenThreads() {
        IntStream.range(0, 10).forEach(i -> performCallback());
    }

    public static void main(String[] args) {
        for (long i = 0; i < Long.MAX_VALUE; i++) {
            if (i % 100000 == 0) {
                System.out.println(i);
            }
            new MySecondTest();
        }
    }
}
