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
package org.astonbitecode.j4rs.api.jfx;

import javafx.application.Application;
import org.astonbitecode.j4rs.api.invocation.NativeCallbackToRustChannelSupport;

public class FxApplicationStartCallback extends NativeCallbackToRustChannelSupport {
    public void setCallbackToApplicationAndLaunch() {
        FxApplication.setCallback(this);
        new Thread(() -> Application.launch(FxApplication.class)).start();
    }

}
