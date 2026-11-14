package dev.basalt.client;

import net.minecraft.client.MinecraftClient;
import net.minecraft.client.network.AbstractClientPlayerEntity;
import net.minecraft.client.texture.NativeImage;
import net.minecraft.client.texture.NativeImageBackedTexture;
import net.minecraft.client.util.SkinTextures;
import net.minecraft.util.Identifier;

import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Locale;
import java.util.UUID;

public final class BasaltSkinManager {
	private static final Identifier LOCAL_SKIN_ID = Identifier.of("basalt", "skins/local");
	private static final String BUNDLED_DEFAULT_SKIN_RESOURCE = "/assets/basalt/skins/default.png";
	private static boolean loadAttempted = false;
	private static boolean loaded = false;
	private static final SkinTextures.Model TARGET_MODEL = readTargetSkinModel();

	private BasaltSkinManager() {
	}

	public static SkinTextures.Model getTargetSkinModel() {
		return TARGET_MODEL;
	}

	public static Identifier getActiveSkinIdIfLocal(AbstractClientPlayerEntity entity) {
		MinecraftClient client = MinecraftClient.getInstance();
		if (client == null || entity == null) {
			return null;
		}
		boolean isSelf = entity == client.player;
		if (!isSelf && entity.getGameProfile() != null && client.getSession() != null) {
			UUID sessionUuid = client.getSession().getUuidOrNull();
			String sessionName = client.getSession().getUsername();
			UUID entityUuid = entity.getGameProfile().getId();
			String entityName = entity.getGameProfile().getName();
			isSelf = (sessionUuid != null && sessionUuid.equals(entityUuid))
					|| (sessionName != null && sessionName.equals(entityName));
		}
		if (!isSelf) {
			return null;
		}
		ensureLoaded(client);
		return loaded ? LOCAL_SKIN_ID : null;
	}

	private static SkinTextures.Model readTargetSkinModel() {
		String raw = System.getProperty("basalt.skinModel");
		if (raw == null || raw.isBlank()) {
			raw = System.getenv("BASALT_SKIN_MODEL");
		}
		if (raw != null) {
			String norm = raw.trim().toLowerCase(Locale.ROOT);
			if ("wide".equals(norm) || "classic".equals(norm)) {
				return SkinTextures.Model.WIDE;
			}
		}
		return SkinTextures.Model.SLIM;
	}

	private static NativeImage readConfiguredOrBundledImage() throws IOException {
		String rawPath = System.getProperty("basalt.skinPath");
		if (rawPath == null || rawPath.isBlank()) {
			rawPath = System.getenv("BASALT_SKIN_PATH");
		}
		if (rawPath != null && !rawPath.isBlank()) {
			Path skinPath = Path.of(rawPath.trim());
			if (Files.isRegularFile(skinPath)) {
				BasaltClientMod.LOGGER.info(
						"Yerel Basalt skini yukleniyor: {} (model: {})",
						skinPath.getFileName(),
						TARGET_MODEL.getName()
				);
				return NativeImage.read(Files.readAllBytes(skinPath));
			}
			BasaltClientMod.LOGGER.warn("Yerel skin dosyasi bulunamadi, varsayilan skine donuluyor: {}", skinPath);
		}

		try (InputStream stream = BasaltSkinManager.class.getResourceAsStream(BUNDLED_DEFAULT_SKIN_RESOURCE)) {
			if (stream == null) {
				BasaltClientMod.LOGGER.warn("Dahili varsayilan skin kaynagi bulunamadi: {}", BUNDLED_DEFAULT_SKIN_RESOURCE);
				return null;
			}
			BasaltClientMod.LOGGER.info(
					"Dahili varsayilan Basalt skini yukleniyor (model: {})",
					TARGET_MODEL.getName()
			);
			return NativeImage.read(stream);
		}
	}

	private static void ensureLoaded(MinecraftClient client) {
		if (loadAttempted) {
			return;
		}
		loadAttempted = true;

		try {
			NativeImage image = readConfiguredOrBundledImage();
			if (image == null) {
				return;
			}
			NativeImageBackedTexture texture = new NativeImageBackedTexture(image);
			client.getTextureManager().registerTexture(LOCAL_SKIN_ID, texture);
			loaded = true;
		} catch (Exception ex) {
			BasaltClientMod.LOGGER.error("Yerel skin yuklenirken hata olustu", ex);
		}
	}
}
