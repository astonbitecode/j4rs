package org.astonbitecode.j4rs.tests;

import org.astonbitecode.j4rs.api.invocation.NativeCallbackToRustChannelSupport;

public class MySecondTest extends NativeCallbackToRustChannelSupport {
    public void performCallback() {
        new Thread(() -> {
            doCallback("THIS IS FROM CALLBACK TO A CHANNEL...");
        }).start();
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
