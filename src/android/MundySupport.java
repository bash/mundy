package garden.tau.mundy;

import static android.content.Context.UI_MODE_SERVICE;

import android.app.UiModeManager;
import android.content.ContentResolver;
import android.content.Context;
import android.content.res.Configuration;
import android.os.Build;
import android.provider.Settings;
import android.util.Log;

/**
 * Support class enabling the Rust crate netwatcher to monitor network interface changes,
 * functionality which is not available in the NDK. This class will be instantiated automatically
 * via JNI and should not be used directly.
 */
public class MundySupport {
    private static final String TAG = "MundySupport";
    private ContentResolver contentResolver;
    private Context context;

    /**
     * Invoked only via JNI.
     * @param context Activity or other Context that can be used to get system services
     */
    public MundySupport(Context context) {
        this.context = context;
        this.contentResolver = context.getContentResolver();
    }

    public void subscribe() {
        Log.v(TAG, "subscribe called");
    }

    public void unsubscribe() {
        Log.v(TAG, "unsubscribe called");
    }

    private native void onPreferencesChanged();
}
