/*
 * Copyright 2020 astonbitecode
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

import javafx.scene.layout.StackPane;
import org.astonbitecode.j4rs.api.Instance;
import org.astonbitecode.j4rs.utils.ChildDummy;
import org.junit.Test;

public class InstanceGeneratorTest {
    @Test
    public void generateCorrectImpl() {
        Instance<?> nonJavaFxInstance = InstanceGenerator.create(new ChildDummy(), ChildDummy.class);
        assert (nonJavaFxInstance.getClass().equals(JsonInvocationImpl.class));

        Instance<?> javaFxInstance = InstanceGenerator.create(new StackPane(), StackPane.class);
        assert (javaFxInstance.getClass().equals(JavaFxInvocation.class));
    }
}
