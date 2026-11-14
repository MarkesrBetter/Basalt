package dev.basalt.client;

import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.rendering.v1.HudLayerRegistrationCallback;
import net.fabricmc.fabric.api.client.rendering.v1.IdentifiedLayer;
import net.minecraft.client.MinecraftClient;
import net.minecraft.client.gui.DrawContext;
import net.minecraft.client.render.RenderTickCounter;
import net.minecraft.util.Identifier;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class BasaltClientMod implements ClientModInitializer {
	public static final String MOD_ID = BasaltCommon.MOD_ID;
	public static final Logger LOGGER = LoggerFactory.getLogger(MOD_ID);
	private static final Identifier FPS_HUD_LAYER_ID = Identifier.of(MOD_ID, "fps_hud");

	@Override
	public void onInitializeClient() {
		boolean showHud = readHudPreference();
		LOGGER.info("Basalt Fabric 1.21.4 client helper ready (FPS HUD: {})", showHud);

		if (showHud) {
			HudLayerRegistrationCallback.EVENT.register(drawer ->
					drawer.attachLayerAfter(IdentifiedLayer.MISC_OVERLAYS, FPS_HUD_LAYER_ID, BasaltClientMod::renderHud)
			);
		}
	}

	private static void renderHud(DrawContext drawContext, RenderTickCounter tickCounter) {
		MinecraftClient client = MinecraftClient.getInstance();
		if (client == null || client.options.hudHidden || client.getDebugHud().shouldShowDebugHud()) {
			return;
		}
		if (client.textRenderer == null) {
			return;
		}
		String user = client.getSession() != null ? client.getSession().getUsername() : "Player";
		String text = client.getCurrentFps() + " FPS  |  " + user;
		int textWidth = client.textRenderer.getWidth(text);
		drawContext.fill(6, 6, 16 + textWidth, 20, 0x96111317);
		drawContext.drawTextWithShadow(client.textRenderer, text, 11, 9, 0xFFEDF1F7);
	}

	private static boolean readHudPreference() {
		String prop = System.getProperty("basalt.showFpsHud");
		if (prop == null || prop.isBlank()) {
			prop = System.getenv("BASALT_SHOW_FPS_HUD");
		}
		if (prop == null || prop.isBlank()) {
			return true;
		}
		return !"false".equalsIgnoreCase(prop.trim());
	}
}
