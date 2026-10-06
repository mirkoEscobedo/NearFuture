package nf.contract.canonical;

import java.security.GeneralSecurityException;
import java.security.KeyFactory;
import java.security.Signature;
import java.security.spec.EdECPrivateKeySpec;
import java.security.spec.NamedParameterSpec;
import java.security.spec.X509EncodedKeySpec;

/** Maintained Bouncy Castle strict prime-subgroup admission and JDK Ed25519 primitive over an already validated 32-byte canonical digest.
 * Key ownership, authorization and policy are supplied by later lifecycle/identity boundaries. */
public final class Ed25519 {
    private Ed25519() { }
    // Standard RFC8410 SubjectPublicKeyInfo header for a 32-byte Ed25519 public key.
    private static final byte[] PUBLIC_HEADER = {0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0};
    public static boolean verify(byte[] publicKey, byte[] canonicalDigest, byte[] signature) {
        if (publicKey.length != 32 || canonicalDigest.length != 32 || signature.length != 64) return false;
        if (!org.bouncycastle.math.ec.rfc8032.Ed25519.validatePublicKeyFull(publicKey, 0)
                || !org.bouncycastle.math.ec.rfc8032.Ed25519.validatePublicKeyFull(signature, 0)) return false;
        byte[] encoded = new byte[PUBLIC_HEADER.length + publicKey.length];
        System.arraycopy(PUBLIC_HEADER, 0, encoded, 0, PUBLIC_HEADER.length);
        System.arraycopy(publicKey, 0, encoded, PUBLIC_HEADER.length, publicKey.length);
        try {
            Signature verifier = Signature.getInstance("Ed25519");
            verifier.initVerify(KeyFactory.getInstance("Ed25519").generatePublic(new X509EncodedKeySpec(encoded)));
            verifier.update(canonicalDigest);
            return verifier.verify(signature);
        } catch (GeneralSecurityException malformed) { return false; }
    }
    public static byte[] sign(byte[] seed, byte[] canonicalDigest) {
        if (seed.length != 32 || canonicalDigest.length != 32) throw new Validation(Validation.Code.INVALID_VALUE);
        try {
            Signature signer = Signature.getInstance("Ed25519");
            signer.initSign(KeyFactory.getInstance("Ed25519").generatePrivate(new EdECPrivateKeySpec(NamedParameterSpec.ED25519, seed)));
            signer.update(canonicalDigest);
            return signer.sign();
        } catch (GeneralSecurityException unavailable) { throw new Validation(Validation.Code.INVALID_VALUE); }
    }
}
