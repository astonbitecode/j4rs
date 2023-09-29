/*
 * Copyright 2023 astonbitecode
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
package org.astonbitecode.j4rs.api.async;

import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.Future;

/**
 * A {@link CompletableFuture} that completes by polling a {@link Future}.
 * 
 * @param <T>
 */
public class J4rsPolledFuture<T> extends CompletableFuture<T> {
    private Future<T> future;

    public J4rsPolledFuture(Future<T> future) {
        this.future = future;
        J4rsAsyncContext.schedule(this::tryToComplete);
    }

    private void tryToComplete() {
        if (future.isDone()) {
            try {
                complete(future.get());
            } catch (InterruptedException error) {
                completeExceptionally(error);
            } catch (ExecutionException error) {
                completeExceptionally(error.getCause());
            }
            return;
        }

        if (future.isCancelled()) {
            cancel(true);
            return;
        }

        J4rsAsyncContext.schedule(this::tryToComplete);
    }
}
