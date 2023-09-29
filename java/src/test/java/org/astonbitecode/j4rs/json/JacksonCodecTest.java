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
package org.astonbitecode.j4rs.json;

import org.astonbitecode.j4rs.api.services.json.exceptions.JsonCodecException;
import org.astonbitecode.j4rs.utils.Dummy;
import org.astonbitecode.j4rs.utils.OtherDummy;
import org.junit.Test;

public class JacksonCodecTest {
    private JacksonCodec jacksonCodec = new JacksonCodec();

    @Test
    public void decodeSuccess() {
        String json = "{\"i\":3}";
        Dummy dummy = jacksonCodec.decode(json, "org.astonbitecode.j4rs.utils.Dummy");
        assert (dummy.getI() == 3);
    }

    @Test(expected = JsonCodecException.class)
    public void decodeFailureWrongClassName() {
        String json = "{\"i\":3}";
        jacksonCodec.decode(json, "org.astonbitecode.j4rs.utils.Nothing");
    }

    @Test(expected = JsonCodecException.class)
    public void decodeFailureInvalidJson() {
        String json = "{mb}";
        jacksonCodec.decode(json, "org.astonbitecode.j4rs.utils.Dummy");
    }

    @Test(expected = JsonCodecException.class)
    public void decodeFailureInvalidMapping() {
        String json = "{\"i\":3}";
        jacksonCodec.decode(json, "org.astonbitecode.j4rs.utils.OtherDummy");
    }

    @Test
    public void encodeSuccess() {
        String json = "{\"i\":3,\"j\":33}";
        OtherDummy person = new OtherDummy(3, 33);
        assert (jacksonCodec.encode(person).equals(json));
    }

    @Test
    public void encodeJ4rsArray() {
        String json = "[\n"
                + "     {\"Rust\":{\"json\":\"\\\"arg1\\\"\",\"class_name\":\"java.lang.String\",\"arg_from\":\"rust\"}},\n"
                + "     {\"Rust\":{\"json\":\"\\\"arg2\\\"\",\"class_name\":\"java.lang.String\",\"arg_from\":\"rust\"}},\n"
                + "     {\"Rust\":{\"json\":\"\\\"arg3\\\"\",\"class_name\":\"java.lang.String\",\"arg_from\":\"rust\"}}\n"
                + "     ]";
        Object[] ret = jacksonCodec.decodeArrayContents(json);
    }

    @Test(expected = JsonCodecException.class)
    public void encodeJ4rsArrayError() {
        String json = "[{\"i\":3,\"j\":33}, {\"i\":333,\"j\":3333}]";
        jacksonCodec.decodeArrayContents(json);
    }
}
