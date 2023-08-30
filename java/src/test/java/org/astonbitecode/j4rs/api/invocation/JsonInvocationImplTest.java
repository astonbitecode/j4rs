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
package org.astonbitecode.j4rs.api.invocation;

import org.astonbitecode.j4rs.api.Instance;
import org.astonbitecode.j4rs.api.dtos.InvocationArg;
import org.astonbitecode.j4rs.api.instantiation.NativeInstantiationImpl;
import org.astonbitecode.j4rs.errors.InvocationException;
import org.astonbitecode.j4rs.tests.MyTest;
import org.astonbitecode.j4rs.utils.*;
import org.junit.Test;

import java.util.concurrent.atomic.AtomicReference;

import static org.mockito.Mockito.*;

public class JsonInvocationImplTest {

    @Test
    public void genericMethodMatches() {
        JsonInvocationImpl toTest = new JsonInvocationImpl(new ChildDummy(), ChildDummy.class);

        Instance invocation1 = toTest.invoke("invokeGeneric", new InvocationArg(new JsonInvocationImpl("astring", String.class)));
        assert (invocation1.getObject().equals(String.class));
    }

    @Test
    public void methodOfGenericInterfaceMatches() {
        Instance childDummy = NativeInstantiationImpl.instantiate("org.astonbitecode.j4rs.utils.ChildDummy");

        Instance toTest = childDummy.invoke("getMap");

        toTest.invoke("put",
                new InvocationArg(new JsonInvocationImpl("three", String.class)),
                new InvocationArg(new JsonInvocationImpl(3, Integer.class)));

        Instance invocation = toTest.invoke("size");
        assert (invocation.getObject().getClass().equals(Integer.class));
        assert (((Integer) invocation.getObject()) == 3);
    }

    // TODO: Is there a way to make this test fail? It should fail theoretically because the Map is defined with generics
    // TODO: <String, Object> and here we put to the Map <Object, String>
    @Test
    public void methodOfGenericInterfaceShouldNotMatch() {
        Instance childDummy = NativeInstantiationImpl.instantiate("org.astonbitecode.j4rs.utils.ChildDummy");

        Instance toTest = childDummy.invoke("getMap");

        toTest.invoke("put",
                new InvocationArg(new JsonInvocationImpl(3, Integer.class)),
                new InvocationArg(new JsonInvocationImpl("three", String.class)));
    }

    @Test
    public void interfaceMethodMatches() {
        JsonInvocationImpl toTest = new JsonInvocationImpl(new DummyMapImpl(), DummyMapInterface.class);

        Instance invocation = toTest.invoke("keysLength");
        assert (invocation.getObject().getClass().equals(Long.class));
        assert (((Long) invocation.getObject()) == 6L);
    }

    @Test
    public void parentInterfaceMethodMatches() {
        JsonInvocationImpl toTest = new JsonInvocationImpl(new DummyMapImpl(), DummyMapInterface.class);

        Instance invocation = toTest.invoke("size");
        assert (invocation.getObject().getClass().equals(Integer.class));
        assert (((Integer) invocation.getObject()) == 2);
    }

    @Test
    public void methodMatches() {
        JsonInvocationImpl toTest = new JsonInvocationImpl(new Dummy(33), Dummy.class);

        Instance invocation = toTest.invoke("getI");
        assert (invocation.getObject().getClass().equals(Integer.class));
        assert (((Integer) invocation.getObject()) == 33);
    }

    @Test
    public void staticMethodMatches() {
        JsonInvocationImpl toTest = new JsonInvocationImpl(DummyWithStatic.class);

        Instance invocation = toTest.invokeStatic("method");
        assert (invocation.getObject().getClass().equals(String.class));
        assert (((String) invocation.getObject()).equals("method product"));
    }

    @Test
    public void withArg() {
        JsonInvocationImpl toTest = new JsonInvocationImpl(new Dummy(33), Dummy.class);

        InvocationArg arg = new InvocationArg(new JsonInvocationImpl(3, Integer.class));
        toTest.invoke("setI", arg);

        Instance getIResult = toTest.invoke("getI");
        assert (getIResult.getObject().getClass().equals(Integer.class));
        assert (((Integer) getIResult.getObject()) == 3);
    }

    @Test
    public void staticMethodWithArg() {
        JsonInvocationImpl toTest = new JsonInvocationImpl(DummyWithStatic.class);

        InvocationArg arg = new InvocationArg(new JsonInvocationImpl(3, Integer.class));
        Instance invocation = toTest.invokeStatic("methodWithArg", arg);

        assert (invocation.getObject().getClass().equals(String.class));
        assert (((String) invocation.getObject()).equals("3"));
    }

    @Test(expected = Exception.class)
    public void noMethodFound() {
        new JsonInvocationImpl(new Dummy(33), Dummy.class).invoke("nonExisting");
    }

    @Test(expected = Exception.class)
    public void errorDuringInvocation() {
        new JsonInvocationImpl(new FailingDummy(), FailingDummy.class).invoke("throwException");
    }

    @Test
    public void cast() {
        Instance from = new JsonInvocationImpl(new ChildDummy(), ChildDummy.class);
        Instance casted = Instance.<Dummy>cast(from, Dummy.class.getName());
//        assert(casted.getObject().getClass().equals(Dummy.class));
    }

    @Test(expected = InvocationException.class)
    public void castFailure() {
        Instance from = new JsonInvocationImpl(new ChildDummy(), ChildDummy.class);
        Instance.cast(from, Integer.class.getName());
    }

    @Test
    public void invokeMethodInHierarchy() {
        Instance ni = new JsonInvocationImpl(new GrandchildDummy(), GrandchildDummy.class);
        Instance res = ni.invoke("getI");
        Integer i = (Integer) res.getObject();
        assert (i.equals(33));
    }

    @Test
    public void invokeMethodInInterface() {
        Instance ni = new JsonInvocationImpl(new GrandchildDummy(), GrandchildDummy.class);
        ni.invoke("doSomething");
    }

    @Test
    public void invokeMethodInInterfaceHierarchy() {
        Instance ni = new JsonInvocationImpl(new GrandchildDummy(), GrandchildDummy.class);
        ni.invoke("doSomethingMoreMom");
        ni.invoke("doSomethingMoreDad");
    }

    @Test(expected = Exception.class)
    public void invokeMethodNotFoundInHierarchy() {
        Instance ni = new JsonInvocationImpl(new GrandchildDummy(), GrandchildDummy.class);
        ni.invoke("nonExisting");
    }

    @Test
    public void getFieldInstance() {
        Instance ni = new JsonInvocationImpl(new DummyWithFields(), DummyWithFields.class);

        Instance res1 = ni.field("pubInt");
        Integer i1 = (Integer) res1.getObject();
        assert (i1.equals(11));

        try {
            Instance res2 = ni.field("packageInt");
            res2.getObject();
            assert (false);
        } catch (InvocationException ie) {
            assert (true);
        }

        try {
            Instance res3 = ni.field("protectedInt");
            res3.getObject();
            assert (false);
        } catch (InvocationException ie) {
            assert (true);
        }

        try {
            Instance res4 = ni.field("privateInt");
            res4.getObject();
            assert (false);
        } catch (InvocationException ie) {
            assert (true);
        }
    }

    @Test
    public void getFieldInstanceInHierarchy() {
        Instance ni = new JsonInvocationImpl(new ChildOfDummyWithFields(), ChildOfDummyWithFields.class);

        Instance res1 = ni.field("pubInt");
        Integer i1 = (Integer) res1.getObject();
        assert (i1.equals(11));

        try {
            Instance res2 = ni.field("packageInt");
            res2.getObject();
            assert (false);
        } catch (InvocationException ie) {
            assert (true);
        }

        try {
            Instance res3 = ni.field("protectedInt");
            res3.getObject();
            assert (false);
        } catch (InvocationException ie) {
            assert (true);
        }

        try {
            Instance res4 = ni.field("privateInt");
            res4.getObject();
            assert (false);
        } catch (InvocationException ie) {
            assert (true);
        }
    }

    @Test
    public void invokeMethodWithArgumentOfSubclassType() {
        Instance instance = new JsonInvocationImpl(new ClassWithDummyAtConstructor(new ChildDummy()), ClassWithDummyAtConstructor.class);

        Instance otherChildDummyInstance = new JsonInvocationImpl(new ChildDummy(), ChildDummy.class);
        instance.invoke("replaceDummy", new InvocationArg(otherChildDummyInstance));
    }

    @Test
    public void invokeAsyncSuccess() throws InterruptedException {
        JsonInvocationImpl instance = spy(new JsonInvocationImpl(new MyTest(), MyTest.class));
        TestCallback callback = new TestCallback();
        doReturn(callback).when(instance).newCallbackForAsyncToChannel(anyLong());
        String testString = "j4rs";
        instance.invokeAsyncToChannel(123, "getStringWithFuture", new InvocationArg(new JsonInvocationImpl(testString, String.class)));

        int i = 1000;
        while (callback.getString() == null && --i > 0) {
            Thread.sleep(100);
        }

        assert (testString.equals(callback.getString()));
    }

    @Test
    public void invokeAsyncFailure() throws InterruptedException {
        JsonInvocationImpl instance = spy(new JsonInvocationImpl(new MyTest(), MyTest.class));
        TestCallback callback = new TestCallback();
        doReturn(callback).when(instance).newCallbackForAsyncToChannel(anyLong());
        String errorString = "Boom!";
        instance.invokeAsyncToChannel(123, "getErrorWithFuture", new InvocationArg(new JsonInvocationImpl(errorString, String.class)));

        int i = 1000;
        while (callback.getString() == null && --i > 0) {
            Thread.sleep(100);
        }

        assert (errorString.equals(callback.getString()));
    }

    private class TestCallback extends NativeCallbackToRustFutureSupport {
        private AtomicReference<String> s = new AtomicReference<>(null);

        @Override
        public void doCallbackSuccess(Object obj) {
            this.s.getAndSet(obj.toString());
        }

        @Override
        public void doCallbackFailure(Throwable error) {
            this.s.getAndSet(error.getMessage());
        }

        public String getString() {
            return this.s.get();
        }
    }
}
