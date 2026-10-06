package nf.adapter;

import com.fs.starfarer.api.BaseModPlugin;
import nf.contract.BuildIdentity;

/** Minimal compile-time API seam. No authority, background work, or game mutation. */
public final class NearFutureModPlugin extends BaseModPlugin {
    public String syntheticBuildIdentity() {
        return BuildIdentity.description();
    }
}
