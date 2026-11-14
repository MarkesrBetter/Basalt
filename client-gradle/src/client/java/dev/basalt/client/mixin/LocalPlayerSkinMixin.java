package dev.basalt.client.mixin;

import dev.basalt.client.BasaltSkinManager;
import net.minecraft.client.network.AbstractClientPlayerEntity;
import net.minecraft.client.util.SkinTextures;
import net.minecraft.util.Identifier;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

@Mixin(AbstractClientPlayerEntity.class)
public abstract class LocalPlayerSkinMixin {
	@Inject(method = "getSkinTextures", at = @At("RETURN"), cancellable = true)
	private void basalt$overrideLocalSkin(CallbackInfoReturnable<SkinTextures> cir) {
		Identifier localSkin = BasaltSkinManager.getActiveSkinIdIfLocal((AbstractClientPlayerEntity) (Object) this);
		if (localSkin == null) {
			return;
		}
		SkinTextures original = cir.getReturnValue();
		if (original == null) {
			return;
		}
		cir.setReturnValue(new SkinTextures(
				localSkin,
				null,
				original.capeTexture(),
				original.elytraTexture(),
				BasaltSkinManager.getTargetSkinModel(),
				original.secure()
		));
	}
}
