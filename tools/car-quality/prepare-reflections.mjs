import sharp from 'sharp';
await sharp('local-test-assets/quality-upgrade/studio_small_09.jpg').resize(2048,1024).png().toFile('assets/envi/car-reflections.png');
await sharp('local-test-assets/quality-upgrade/studio_small_09.jpg').resize(512,256).blur(30).png().toFile('assets/envi/car-reflections-blur.png');
