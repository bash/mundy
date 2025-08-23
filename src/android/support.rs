use super::result::ArcError;
use super::subscription::Java_garden_tau_mundy_MundySupport_onPreferencesChanged;
use jni::objects::{GlobalRef, JClass, JObject, JValue};
use jni::{JNIEnv, JavaVM, NativeMethod};
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

type Result<T, E = BoxedError> = std::result::Result<T, E>;
type BoxedError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Clone)]
pub(crate) struct JavaSupport {
    global_ref: GlobalRef,
}

impl JavaSupport {
    pub(crate) fn get() -> Result<Self> {
        static INSTANCE: LazyLock<Result<JavaSupport, ArcError>> =
            LazyLock::new(|| JavaSupport::from_android_context().map_err(ArcError::from));
        INSTANCE.clone().map_err(Into::into)
    }

    fn from_android_context() -> Result<Self> {
        let vm = java_vm()?;
        let mut env = vm.attach_current_thread()?;
        let context = android_content_context();
        let class = inject_dex_class(&mut env, &context)?;
        let instance = env.new_object(
            &class,
            "(Landroid/content/Context;)V",
            &[JValue::from(&context)],
        )?;
        let global_ref = env.new_global_ref(instance)?;
        Ok(Self { global_ref })
    }

    #[cfg(feature = "color-scheme")]
    pub(crate) fn get_night_mode(&self, env: &mut JNIEnv) -> Result<bool> {
        Ok(env
            .call_method(&self.global_ref, "getNightMode", "()Z", &[])?
            .z()
            .expect("method to return a boolean"))
    }

    #[cfg(feature = "contrast")]
    pub(crate) fn get_high_contrast(&self, env: &mut JNIEnv) -> Result<bool> {
        Ok(env
            .call_method(&self.global_ref, "getHighContrast", "()Z", &[])?
            .z()
            .expect("method to return a boolean"))
    }

    pub(crate) fn subscribe(&self, env: &mut JNIEnv) -> Result<()> {
        env.call_method(&self.global_ref, "subscribe", "()V", &[])?;
        Ok(())
    }

    pub(crate) fn unsubscribe(&self, env: &mut JNIEnv) -> Result<()> {
        env.call_method(&self.global_ref, "unsubscribe", "()V", &[])?;
        Ok(())
    }
}

pub(crate) fn java_vm() -> Result<JavaVM> {
    let ctx = ndk_context::android_context();
    // SAFETY: ndk_context gives us a valid pointer.
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }?;
    Ok(vm)
}

pub(crate) fn android_content_context<'local>() -> JObject<'local> {
    let ctx = ndk_context::android_context();
    // SAFETY: ndk_context gives us a valid pointer.
    unsafe { JObject::from_raw(ctx.context().cast()) }
}

// This is again adapted from netwatcher's source:
// <https://github.com/thombles/netwatcher/blob/f1353ba6b9a9e4e28a223a317564a3b34a649aae/src/watch_android.rs#L94>
fn inject_dex_class<'a>(
    env: &mut JNIEnv<'a>,
    context_obj: &jni::objects::JObject,
) -> Result<JClass<'a>> {
    const MUNDY_DEX_BYTES: &[u8] = include_bytes!(env!("MUNDY_DEX_PATH"));

    // to enable backwards compat to API level 21, write to disk instead of loading in-memory
    let cache_dir = env.call_method(context_obj, "getCodeCacheDir", "()Ljava/io/File;", &[])?;
    let cache_dir_path = env.call_method(
        &cache_dir.l()?,
        "getAbsolutePath",
        "()Ljava/lang/String;",
        &[],
    )?;
    let cache_dir_jstring = cache_dir_path.l()?;
    let cache_dir_rust: String = env.get_string(&cache_dir_jstring.into())?.into();
    let temp_dex_path = PathBuf::from(cache_dir_rust.clone()).join("mundy.dex");
    fs::write(&temp_dex_path, MUNDY_DEX_BYTES)?;

    // dex file must not be writable or it won't be loaded
    let mut perms = fs::metadata(&temp_dex_path)?.permissions();
    perms.set_readonly(true);
    fs::set_permissions(&temp_dex_path, perms)?;

    let dex_class_loader_class = env.find_class("dalvik/system/DexClassLoader")?;
    let parent_loader = env.call_method(
        context_obj,
        "getClassLoader",
        "()Ljava/lang/ClassLoader;",
        &[],
    )?;

    let temp_dex_path_str = temp_dex_path.to_string_lossy().to_string();
    let temp_dex_path_jstring = env.new_string(&temp_dex_path_str)?;
    let cache_dir_jstring = env.new_string(&cache_dir_rust)?;
    let dex_loader = env.new_object(
        &dex_class_loader_class,
        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/ClassLoader;)V",
        &[
            (&temp_dex_path_jstring).into(),
            (&cache_dir_jstring).into(),
            (&JObject::null()).into(),
            (&parent_loader.l()?).into(),
        ],
    )?;

    let class_name_str = env.new_string("garden.tau.mundy.MundySupport")?;
    let support_class_obj = env.call_method(
        &dex_loader,
        "loadClass",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        &[(&class_name_str).into()],
    )?;
    let support_class: JClass = support_class_obj.l()?.into();
    let _ = fs::remove_file(&temp_dex_path);

    let native_methods = [NativeMethod {
        name: "onPreferencesChanged".into(),
        sig: "()V".into(),
        fn_ptr: Java_garden_tau_mundy_MundySupport_onPreferencesChanged as *mut _,
    }];
    env.register_native_methods(&support_class, &native_methods)?;

    Ok(support_class)
}
