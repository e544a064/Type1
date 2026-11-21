package fr.type1.keyboard

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Typeface
import android.media.AudioManager
import android.os.Handler
import android.os.Looper
import android.util.AttributeSet
import android.view.HapticFeedbackConstants
import android.view.MotionEvent
import android.view.View
import fr.type1.keyboard.core.KeyboardController
import fr.type1.keyboard.model.KeyAction
import fr.type1.keyboard.model.LetterKey
import fr.type1.keyboard.model.RowModel
import fr.type1.keyboard.model.SUGGESTION_PREFIX
import fr.type1.keyboard.model.UtilityKey
import kotlin.math.abs
import kotlin.math.min

/**
 * Three horizontal bands, driven entirely by [RowModel]:
 *
 * 1. suggestion strip (shift, suggestions, backspace)
 * 2. every letter on one line, no groups, row-based vertical offsets
 * 3. utility keys (page, space, comma, enter)
 */
@SuppressLint("ClickableViewAccessibility", "ViewConstructor")
class KeyboardView(
	context: Context,
	private val controller: KeyboardController,
	attrs: AttributeSet? = null,
) : View(context, attrs) {

	interface Listener {
		fun onActions(actions: List<KeyAction>)
	}

	var listener: Listener? = null

	private enum class Band { None, Suggestions, Letters, Utilities }

	private class AltState(val alts: List<String>) {
		var selected: Int = -1
	}

	private val handler = Handler(Looper.getMainLooper())
	private val audio = context.getSystemService(Context.AUDIO_SERVICE) as AudioManager

	private var model: RowModel? = null

	private val suggestionHeight = dp(38f)
	private val letterHeight = dp(56f)
	private val utilityHeight = dp(48f)
	private val bandGap = dp(3f)
	private val padding = dp(3f)
	private val corner = dp(8f)

	private val suggestionTop = padding
	private val letterTop = padding + suggestionHeight + bandGap
	private val utilityTop = letterTop + letterHeight + bandGap
	private val contentHeight = utilityTop + utilityHeight + padding

	/** Height of the system navigation bar overlapping the IME window. */
	private var bottomInset = 0f
	private val totalHeight: Float get() = contentHeight + bottomInset

	private val stripLeft = RectF()
	private val stripRight = RectF()
	private val suggestionRects = ArrayList<RectF>()
	private val letterRects = ArrayList<RectF>()
	private var letterTextSize = dp(16f)
	private val utilityRects = ArrayList<RectF>()
	private val altRects = ArrayList<RectF>()

	private var band = Band.None
	private var pressedLetter = -1
	private var pressedUtility = -1
	private var pressedSuggestion = -1
	private var pressedStrip = -1
	private var downUtility = -1
	private var altState: AltState? = null
	private var longPressFired = false

	private val backgroundPaint = Paint().apply {
		color = context.getColor(R.color.keyboard_background)
	}
	private val keyPaint = Paint(Paint.ANTI_ALIAS_FLAG)
	private val labelPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
		typeface = Typeface.create("sans-serif-medium", Typeface.NORMAL)
		textAlign = Paint.Align.CENTER
	}

	private val colors = Colors(
		key = context.getColor(R.color.key_background),
		pressed = context.getColor(R.color.key_background_pressed),
		accent = context.getColor(R.color.accent),
		text = context.getColor(R.color.key_text),
		muted = context.getColor(R.color.key_text_muted),
		strip = context.getColor(R.color.suggestion_background),
	)

	private data class Colors(
		val key: Int,
		val pressed: Int,
		val accent: Int,
		val text: Int,
		val muted: Int,
		val strip: Int,
	)

	private val longPressRunnable = Runnable {
		when (band) {
			Band.Letters -> {
				val letter = model?.letters?.getOrNull(pressedLetter) ?: return@Runnable
				if (letter.alts.isNotEmpty()) {
					longPressFired = true
					altState = AltState(letter.alts)
					altRects.clear()
					invalidate()
				}
			}
			Band.Utilities -> {
				val key = model?.keys?.getOrNull(pressedUtility) ?: return@Runnable
				if (key.kind == "backspace") {
					longPressFired = true
					startRepeat()
				}
			}
			Band.Suggestions -> {
				val key = model?.strip?.getOrNull(pressedStrip) ?: return@Runnable
				if (key.kind == "backspace") {
					longPressFired = true
					startRepeat()
				}
			}
			else -> Unit
		}
	}

	private val repeatRunnable = object : Runnable {
		override fun run() {
			emit(controller.press("backspace"))
			handler.postDelayed(this, REPEAT_MS)
		}
	}

	init {
		setBackgroundColor(context.getColor(R.color.keyboard_background))
	}

	/** Re-pull the three bands from the core. */
	fun refresh() {
		val next = controller.render()
		model = next
		altState = null
		altRects.clear()
		layoutBands(next)
		invalidate()
	}

	/** Reserve space for the system navigation bar so it never covers keys. */
	fun setBottomInset(pixels: Float) {
		if (pixels == bottomInset) return
		bottomInset = pixels
		requestLayout()
		invalidate()
	}

	override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
		setMeasuredDimension(MeasureSpec.getSize(widthMeasureSpec), totalHeight.toInt() + 1)
	}

	override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
		super.onSizeChanged(w, h, oldw, oldh)
		model?.let { layoutBands(it) }
	}

	private fun layoutBands(row: RowModel) {
		if (width == 0) return

		stripLeft.set(padding, suggestionTop, padding + suggestionHeight, suggestionTop + suggestionHeight)
		stripRight.set(
			width - padding - suggestionHeight,
			suggestionTop,
			width - padding,
			suggestionTop + suggestionHeight,
		)

		suggestionRects.clear()
		val suggestionsLeft = stripLeft.right + dp(4f)
		val suggestionsRight = stripRight.left - dp(4f)
		if (row.suggestions.isNotEmpty() && suggestionsRight > suggestionsLeft) {
			val each = (suggestionsRight - suggestionsLeft) / row.suggestions.size
			var x = suggestionsLeft
			for (i in row.suggestions.indices) {
				suggestionRects += RectF(x, suggestionTop, x + each, suggestionTop + suggestionHeight)
				x += each
			}
		}

		layoutLetters(row.letters)
		layoutUtilities(row.keys)
	}

	private fun layoutLetters(letters: List<LetterKey>) {
		letterRects.clear()
		if (letters.isEmpty()) return
		val available = width - padding * 2
		if (available <= 0f) return
		val each = available / letters.size
		var x = padding
		for (i in letters.indices) {
			letterRects += RectF(x, letterTop, x + each, letterTop + letterHeight)
			x += each
		}

		// One size for the whole row, measured on the uppercase labels so it
		// does not jump when shift toggles.
		var size = dp(17f)
		val minimum = dp(9f)
		labelPaint.textSize = size
		while (size > minimum && letters.any { labelPaint.measureText(it.label) > each - dp(1f) }) {
			size -= 0.5f
			labelPaint.textSize = size
		}
		letterTextSize = size
	}

	private fun layoutUtilities(keys: List<UtilityKey>) {
		utilityRects.clear()
		if (keys.isEmpty()) return
		val keyGap = dp(4f)
		val available = width - padding * 2 - keyGap * (keys.size - 1)
		val total = keys.sumOf { it.weight.toDouble() }.toFloat()
		if (available <= 0f || total <= 0f) return
		val unit = available / total
		var x = padding
		for (key in keys) {
			val keyWidth = unit * key.weight
			utilityRects += RectF(x, utilityTop, x + keyWidth, utilityTop + utilityHeight)
			x += keyWidth + keyGap
		}
	}

	override fun onDraw(canvas: Canvas) {
		canvas.drawRect(0f, 0f, width.toFloat(), height.toFloat(), backgroundPaint)
		val row = model ?: return
		drawSuggestionBand(canvas, row)
		drawUtilities(canvas, row)

		val alt = altState
		if (alt != null) {
			drawAlternates(canvas, alt)
		} else {
			drawLetters(canvas, row)
		}
	}

	private fun drawSuggestionBand(canvas: Canvas, row: RowModel) {
		keyPaint.color = colors.strip
		canvas.drawRoundRect(
			padding,
			suggestionTop,
			width - padding,
			suggestionTop + suggestionHeight,
			corner,
			corner,
			keyPaint,
		)

		labelPaint.color = colors.muted
		labelPaint.textSize = suggestionHeight * 0.40f
		drawStripKey(canvas, stripLeft, row.strip.getOrNull(0), 0)
		drawStripKey(canvas, stripRight, row.strip.getOrNull(1), 1)

		for (i in row.suggestions.indices) {
			val rect = suggestionRects.getOrNull(i) ?: continue
			val pressed = pressedSuggestion == i
			if (pressed) {
				keyPaint.color = colors.accent
				canvas.drawRoundRect(rect, corner, corner, keyPaint)
			}
			labelPaint.color = if (pressed) Color.WHITE else colors.text
			labelPaint.textSize = fitText(
				row.suggestions[i],
				rect.width() - dp(8f),
				dp(16f),
				dp(10f),
			)
			canvas.drawText(row.suggestions[i], rect.centerX(), baseline(rect), labelPaint)
		}
	}

	/** Shift on the left, backspace on the right: real keys inside the strip. */
	private fun drawStripKey(canvas: Canvas, rect: RectF, key: UtilityKey?, index: Int) {
		if (key == null) return
		val pressed = pressedStrip == index
		keyPaint.color = when {
			pressed -> colors.pressed
			key.active -> colors.accent
			else -> colors.key
		}
		canvas.drawRoundRect(rect, corner, corner, keyPaint)
		labelPaint.color = if (key.active || pressed) Color.WHITE else colors.muted
		labelPaint.textSize = fitText(
			key.label,
			rect.width() - dp(8f),
			suggestionHeight * 0.44f,
			dp(10f),
		)
		canvas.drawText(key.label, rect.centerX(), baseline(rect), labelPaint)
	}

	private fun drawLetters(canvas: Canvas, row: RowModel) {
		for (i in row.letters.indices) {
			val rect = letterRects.getOrNull(i) ?: continue
			val letter = row.letters[i]
			val dy = rowOffset(letter)
			val pressed = pressedLetter == i
			if (pressed) {
				keyPaint.color = colors.accent
				val half = dp(19f)
				val center = rect.centerY() + dy
				canvas.drawRoundRect(
					RectF(rect.left + dp(1f), center - half, rect.right - dp(1f), center + half),
					dp(6f),
					dp(6f),
					keyPaint,
				)
			}
			labelPaint.color = if (pressed) Color.WHITE else colors.text
			labelPaint.textSize = letterTextSize
			val label = displayLabel(letter, row.caps)
			canvas.drawText(label, rect.centerX(), baseline(rect) + dy, labelPaint)
		}
	}

	/** Uppercase while shift (or auto-capitals) is active, lowercase otherwise. */
	private fun displayLabel(letter: LetterKey, caps: Boolean): String {
		if (!letter.letter) return letter.label
		return if (caps) letter.label.uppercase() else letter.label.lowercase()
	}

	/** Top-row letters sit higher, bottom-row letters lower. */
	private fun rowOffset(letter: LetterKey): Float = when (letter.row) {
		0 -> -dp(7f)
		2 -> dp(7f)
		else -> 0f
	}

	private fun drawUtilities(canvas: Canvas, row: RowModel) {
		for (i in row.keys.indices) {
			val rect = utilityRects.getOrNull(i) ?: continue
			val key = row.keys[i]
			keyPaint.color = when {
				pressedUtility == i -> colors.pressed
				key.kind == "enter" || key.active -> colors.accent
				else -> colors.key
			}
			canvas.drawRoundRect(rect, corner, corner, keyPaint)

			labelPaint.color = when (key.kind) {
				"space", "page" -> colors.muted
				else -> colors.text
			}
			labelPaint.textSize = fitText(key.label, rect.width() - dp(6f), dp(16f), dp(9f))
			canvas.drawText(key.label, rect.centerX(), baseline(rect), labelPaint)
		}
	}

	private fun drawAlternates(canvas: Canvas, alt: AltState) {
		if (altRects.size != alt.alts.size) {
			layoutAlternates(alt.alts.size)
		}
		for (i in alt.alts.indices) {
			val rect = altRects.getOrNull(i) ?: continue
			keyPaint.color = if (alt.selected == i) colors.accent else colors.key
			canvas.drawRoundRect(rect, corner, corner, keyPaint)
			labelPaint.color = colors.text
			labelPaint.textSize = fitText(alt.alts[i], rect.width() - dp(6f), dp(22f), dp(10f))
			canvas.drawText(alt.alts[i], rect.centerX(), baseline(rect), labelPaint)
		}
	}

	private fun layoutAlternates(count: Int) {
		altRects.clear()
		if (count <= 0) return
		val gap = dp(4f)
		val available = width - padding * 2 - gap * (count - 1)
		val each = available / count
		var x = padding
		repeat(count) {
			altRects += RectF(x, letterTop, x + each, letterTop + letterHeight)
			x += each + gap
		}
	}

	private fun baseline(rect: RectF): Float =
	rect.centerY() - (labelPaint.ascent() + labelPaint.descent()) / 2f

	private fun fitText(text: String, maxWidth: Float, startSize: Float, minSize: Float): Float {
		var size = startSize
		labelPaint.textSize = size
		while (size > minSize && labelPaint.measureText(text) > maxWidth) {
			size -= 0.5f
			labelPaint.textSize = size
		}
		return size
	}

	override fun onTouchEvent(event: MotionEvent): Boolean {
		when (event.actionMasked) {
			MotionEvent.ACTION_DOWN -> {
				longPressFired = false
				band = bandAt(event.y)
				when (band) {
					Band.Suggestions -> {
						pressedStrip = when {
							stripLeft.contains(event.x, event.y) -> 0
							stripRight.contains(event.x, event.y) -> 1
							else -> -1
						}
						if (pressedStrip >= 0) {
							performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
							scheduleLongPress()
						} else {
							pressedSuggestion = hit(suggestionRects, event.x)
						}
						invalidate()
					}
					Band.Letters -> {
						pressedLetter = hitLetter(event.x)
						if (pressedLetter >= 0) {
							performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
							scheduleLongPress()
						}
						invalidate()
					}
					Band.Utilities -> {
						downUtility = hit(utilityRects, event.x)
						pressedUtility = downUtility
						if (downUtility >= 0) {
							performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
							scheduleLongPress()
						}
						invalidate()
					}
					Band.None -> Unit
				}
				return true
			}

			MotionEvent.ACTION_MOVE -> {
				val alt = altState
				if (alt != null) {
					updateAlternateSelection(alt, event.x)
					return true
				}
				when (band) {
					Band.Letters -> {
						val index = hitLetter(event.x)
						if (index != pressedLetter) {
							pressedLetter = index
							invalidate()
						}
					}
					Band.Utilities -> {
						val index = hit(utilityRects, event.x)
						if (index != pressedUtility) {
							pressedUtility = index
							invalidate()
						}
					}
					Band.Suggestions -> {
						if (pressedStrip >= 0) {
							val next = when {
								stripLeft.contains(event.x, event.y) -> 0
								stripRight.contains(event.x, event.y) -> 1
								else -> -1
							}
							if (next != pressedStrip) {
								pressedStrip = next
								invalidate()
							}
						} else {
							val index = hit(suggestionRects, event.x)
							if (index != pressedSuggestion) {
								pressedSuggestion = index
								invalidate()
							}
						}
					}
					Band.None -> Unit
				}
				return true
			}

			MotionEvent.ACTION_UP -> {
				cancelTimers()
				val alt = altState
				if (alt != null) {
					val choice = alt.selected
					altState = null
					altRects.clear()
					if (choice in alt.alts.indices) {
						emit(controller.press("alt:${alt.alts[choice]}"))
					}
					resetTouch()
					return true
				}

				when (band) {
					Band.Suggestions -> when {
						pressedStrip >= 0 && !longPressFired -> model?.strip?.getOrNull(pressedStrip)?.let { emit(controller.press(it.id)) }
						pressedSuggestion >= 0 -> emit(controller.press("$SUGGESTION_PREFIX$pressedSuggestion"))
					}
					Band.Letters -> if (!longPressFired && pressedLetter >= 0) {
						model?.letters?.getOrNull(pressedLetter)?.let { emit(controller.press(it.id)) }
					}
					Band.Utilities -> if (!longPressFired && pressedUtility >= 0 && pressedUtility == downUtility) {
						model?.keys?.getOrNull(pressedUtility)?.let { emit(controller.press(it.id)) }
					}
					Band.None -> Unit
				}
				resetTouch()
				return true
			}

			MotionEvent.ACTION_CANCEL -> {
				cancelTimers()
				resetTouch()
			}
		}
		return true
	}

	private fun updateAlternateSelection(alt: AltState, x: Float) {
		val index = altRects.indexOfFirst { x >= it.left && x <= it.right }
		if (index != alt.selected) {
			alt.selected = index
			if (index >= 0) performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
			invalidate()
		}
	}

	private fun bandAt(y: Float): Band = when {
		y < letterTop - bandGap / 2f -> Band.Suggestions
		y < utilityTop - bandGap / 2f -> Band.Letters
		y <= height.toFloat() -> Band.Utilities
		else -> Band.None
	}

	private fun hit(rects: List<RectF>, x: Float): Int {
		for (i in rects.indices) {
			if (x >= rects[i].left && x <= rects[i].right) return i
		}
		return -1
	}

	/** Nearest-letter lookup, so the gaps between groups stay typable. */
	private fun hitLetter(x: Float): Int {
		if (letterRects.isEmpty()) return -1
		val direct = hit(letterRects, x)
		if (direct >= 0) return direct
		var best = -1
		var bestDistance = Float.MAX_VALUE
		for (i in letterRects.indices) {
			val rect = letterRects[i]
			val distance = min(abs(x - rect.left), abs(x - rect.right))
			if (distance < bestDistance) {
				bestDistance = distance
				best = i
			}
		}
		return best
	}

	private fun scheduleLongPress() {
		handler.removeCallbacks(longPressRunnable)
		handler.postDelayed(longPressRunnable, LONG_PRESS_MS)
	}

	private fun emit(actions: List<KeyAction>) {
		if (actions.isNotEmpty()) {
			playClick()
			listener?.onActions(actions)
		}
		refresh()
	}

	private fun startRepeat() {
		handler.post(repeatRunnable)
	}

	private fun cancelTimers() {
		handler.removeCallbacks(longPressRunnable)
		handler.removeCallbacks(repeatRunnable)
	}

	private fun resetTouch() {
		band = Band.None
		pressedLetter = -1
		pressedUtility = -1
		pressedSuggestion = -1
		pressedStrip = -1
		downUtility = -1
		altState = null
		altRects.clear()
		longPressFired = false
		invalidate()
	}

	private fun playClick() {
		try {
			audio.playSoundEffect(AudioManager.FX_KEYPRESS_STANDARD, 0.6f)
		} catch (_: Exception) {
			// Sound is best effort.
		}
	}

	override fun onDetachedFromWindow() {
		handler.removeCallbacksAndMessages(null)
		super.onDetachedFromWindow()
	}

	private fun dp(value: Float): Float = value * resources.displayMetrics.density

	private companion object {
		const val LONG_PRESS_MS = 320L
		const val REPEAT_MS = 55L
	}
}
