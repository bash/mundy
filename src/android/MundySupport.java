package garden.tau.mundy;

import static android.content.Context.UI_MODE_SERVICE;

import android.app.UiModeManager;
import android.content.ContentResolver;
import android.content.Context;
import android.content.res.Configuration;
import android.os.Build;
import android.provider.Settings;
import android.util.Log;
import java.util.concurrent.Executor;
import java.lang.Override;
import android.app.UiModeManager.ContrastChangeListener;
import android.database.ContentObserver;
import android.os.Handler;

/**
 * Support class enabling the Rust crate netwatcher to monitor network interface changes,
 * functionality which is not available in the NDK. This class will be instantiated automatically
 * via JNI and should not be used directly.
 */
public class MundySupport {
    private static final String TAG = "MundySupport";
    private ContentResolver contentResolver;
    private Context context;

    private ContrastChangeListener contrastChangeListener;
    private ContentObserver contentObserver;

    /**
     * Invoked only via JNI.
     * @param context Activity or other Context that can be used to get system services
     */
    public MundySupport(Context context) {
        this.context = context;
        this.contentResolver = context.getContentResolver();
    }

    public void subscribe() {
        try {
            trySubscribe();
        }
        catch (Exception e) {
            Log.w(TAG, "failed to set up subscription", e);
            throw e;
        }
    }

    private void trySubscribe() {
        Log.v(TAG, "subscribe called");

        final boolean notifyForDescendants = false;
        contentObserver = new ContentObserver(MundyBackgroundThread.getHandler()) {
            @Override
            public void onChange(boolean selfChange) {
                super.onChange(selfChange);
                onPreferencesChanged();
            }

            @Override
            public boolean deliverSelfNotifications() {
                return true;
            }
        };
        contentResolver.registerContentObserver(Settings.Secure.getUriFor("high_text_contrast_enabled"), notifyForDescendants, contentObserver);

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            final UiModeManager uiModeManager = (UiModeManager) context.getSystemService(UI_MODE_SERVICE);
            contrastChangeListener = new ContrastChangeListener() {
                public void onContrastChanged(final float contrast) {
                    onPreferencesChanged();
                }
            };
            uiModeManager.addContrastChangeListener(context.getMainExecutor(), contrastChangeListener);
        }
    }

    public void unsubscribe() {
        Log.v(TAG, "unsubscribe called");

        contentResolver.unregisterContentObserver(contentObserver);

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            final UiModeManager uiModeManager = (UiModeManager) context.getSystemService(UI_MODE_SERVICE);
            uiModeManager.removeContrastChangeListener(contrastChangeListener);
        }
    }

    public boolean getNightMode() {
        final int uiMode = context.getResources().getConfiguration().uiMode;
        return (uiMode & Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES;
    }

    // This is how both Firefox & Chromium do it; seems sensible to do it the same.
    // <https://github.com/mozilla-firefox/firefox/blob/ff058e8e75bfdd11a1bdbd1a706c3a4448bce335/mobile/android/geckoview/src/main/java/org/mozilla/gecko/GeckoSystemStateListener.java#L192>
    // <https://source.chromium.org/chromium/chromium/src/+/main:ui/accessibility/android/java/src/org/chromium/ui/accessibility/AccessibilityState.java;l=544;drc=057b542e8c6318874cb4ae6120a601ffdeac9c26>
    public boolean getHighContrast() {
        return getHighTextContrastEnabled() || getContrastLevel() == 1f;
    }

    private boolean getHighTextContrastEnabled() {
        return Settings.Secure.getInt(contentResolver, "high_text_contrast_enabled", 0) == 1;
    }

    private float getContrastLevel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            final UiModeManager uiModeManager = (UiModeManager) context.getSystemService(UI_MODE_SERVICE);
            return uiModeManager.getContrast();
        }
        return 0f;
    }

    private native void onPreferencesChanged();
}
