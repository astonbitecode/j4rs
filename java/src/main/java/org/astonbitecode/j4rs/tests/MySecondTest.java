package org.astonbitecode.j4rs.tests;

import java8.util.stream.IntStreams;
import org.astonbitecode.j4rs.api.invocation.NativeCallbackToRustChannelSupport;

public class MySecondTest extends NativeCallbackToRustChannelSupport {

    public static MyTest myTestFactory() {
        return new MyTest();
    }

    public void performCallback() {
        new Thread(() -> {
            doCallback("THIS IS FROM CALLBACK TO A CHANNEL...");
        }).start();
    }

    public void performTenCallbacks() {
        new Thread(() -> {
            IntStreams.range(0, 10).forEach(i -> doCallback("THIS IS FROM CALLBACK TO A CHANNEL..." + i));
        }).start();
    }

    public void performCallbackFromTenThreads() {
        IntStreams.range(0, 10).forEach(i -> performCallback());
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
