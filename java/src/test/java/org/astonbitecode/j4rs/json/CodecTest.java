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

import com.fasterxml.jackson.core.JsonParseException;
import com.fasterxml.jackson.databind.JsonMappingException;
import org.astonbitecode.j4rs.api.dtos.InvocationArg;
import org.astonbitecode.j4rs.errors.JsonCodecException;
import org.astonbitecode.j4rs.utils.Dummy;
import org.astonbitecode.j4rs.utils.OtherDummy;
import org.junit.Test;

import java.util.Arrays;

public class CodecTest {
    private Codec codec = new Codec();

    @Test
    public void decodeSuccess() throws Exception {
        String json = "{\"i\":3}";
        Dummy dummy = codec.decode(json, "org.astonbitecode.j4rs.utils.Dummy");
        assert (dummy.getI() == 3);
    }

    @Test(expected = ClassNotFoundException.class)
    public void decodeFailureWrongClassName() throws Exception {
        String json = "{\"i\":3}";
        codec.decode(json, "org.astonbitecode.j4rs.utils.Nothing");
    }

    @Test(expected = JsonParseException.class)
    public void decodeFailureInvalidJson() throws Exception {
        String json = "{mb}";
        codec.decode(json, "org.astonbitecode.j4rs.utils.Dummy");
    }

    @Test(expected = JsonMappingException.class)
    public void decodeFailureInvalidMapping() throws Exception {
        String json = "{\"i\":3}";
        codec.decode(json, "org.astonbitecode.j4rs.utils.OtherDummy");
    }

    @Test
    public void encodeSuccess() throws Exception {
        String json = "{\"i\":3,\"j\":33}";
        OtherDummy person = new OtherDummy(3, 33);
        assert (codec.encode(person).equals(json));
    }

    @Test
    public void encodeJ4rsArray() throws Exception {
        String json = "[\n" +
                "     {\"Rust\":{\"json\":\"\\\"arg1\\\"\",\"class_name\":\"java.lang.String\",\"arg_from\":\"rust\"}},\n" +
                "     {\"Rust\":{\"json\":\"\\\"arg2\\\"\",\"class_name\":\"java.lang.String\",\"arg_from\":\"rust\"}},\n" +
                "     {\"Rust\":{\"json\":\"\\\"arg3\\\"\",\"class_name\":\"java.lang.String\",\"arg_from\":\"rust\"}}\n" +
                "     ]";
        Object[] ret = codec.decodeArrayContents(json);
    }

    @Test(expected = JsonCodecException.class)
    public void encodeJ4rsArrayError() throws Exception {
        String json = "[{\"i\":3,\"j\":33}, {\"i\":333,\"j\":3333}]";
        codec.decodeArrayContents(json);
    }
}

