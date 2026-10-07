package nf.wire;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HexFormat;
import java.util.regex.Pattern;
final class AuthWireCorpus {
    private AuthWireCorpus() { }
    static void run() throws java.io.IOException {
        // Fixed public data fixtures only; this helper is not a general JSON or executable validator.
        String json=Files.readString(Path.of("protocol/vectors/local-auth-v1.json"));
        var valid=Pattern.compile("\"name\": \"([a-z_]+)\",\\s*\"wireHex\": \"([0-9a-f]+)\"").matcher(json);
        int positive=0;while(valid.find()) {LocalAuthDecoder.decode(HexFormat.of().parseHex(valid.group(2)));positive++;}
        var invalid=Pattern.compile("\"name\": \"([a-z_]+)\",\\s*\"error\": \"([A-Z_]+)\",\\s*\"wireHex\": \"([0-9a-f]+)\"").matcher(json);
        int negative=0;while(invalid.find()) {AuthWireTest.expect(WireFailure.Code.valueOf(invalid.group(2)),HexFormat.of().parseHex(invalid.group(3)));negative++;}
        if(positive!=4||negative!=14) throw new AssertionError("fixed auth corpus size");
        System.out.println("PASS: shared independent auth wire corpus 4 positive/14 malformed");
    }
}