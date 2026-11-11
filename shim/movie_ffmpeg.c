/* Movies, through FFmpeg's libraries: pictures decoded one at a time as they are wanted, and
 * all the sound at once, through a second reading of the file. */
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/channel_layout.h>
#include <libswresample/swresample.h>
#include <libswscale/swscale.h>
#include <stdlib.h>
#include "oe_movie.h"

struct OeMovie {
    AVFormatContext *format;
    AVCodecContext *video;
    int stream;
    struct SwsContext *scale;
    AVPacket *packet;
    AVFrame *frame;
    int width, height;
    /* Whether the file has been read to its end, and the decoder told so. */
    int draining;
    float *sound;
    uint32_t sound_frames;
};

static AVCodecContext *open_decoder(AVFormatContext *format, int stream) {
    const AVCodecParameters *parameters = format->streams[stream]->codecpar;
    const AVCodec *codec = avcodec_find_decoder(parameters->codec_id);
    if (!codec) return NULL;
    AVCodecContext *decoder = avcodec_alloc_context3(codec);
    if (!decoder || avcodec_parameters_to_context(decoder, parameters) < 0 ||
        avcodec_open2(decoder, codec, NULL) < 0) {
        avcodec_free_context(&decoder);
        return NULL;
    }
    return decoder;
}

/* Sound decoded so far, and how much room there is for it. */
typedef struct {
    float *samples;
    uint32_t frames;
    size_t capacity;
} Sound;

/* Makes room for this many more frames of sound. Returns 0 if there is none. */
static int grow_sound(Sound *sound, int more) {
    size_t needed = (size_t)sound->frames + (size_t)more;
    if (needed <= sound->capacity) return 1;
    size_t bigger = needed * 2;
    float *grown = realloc(sound->samples, bigger * OE_MOVIE_SOUND_CHANNELS * sizeof(float));
    if (!grown) return 0;
    sound->samples = grown;
    sound->capacity = bigger;
    return 1;
}

/* Reads the file through once for its sound, as interleaved float samples. */
static Sound decode_sound(const char *path) {
    Sound sound = {0};
    AVFormatContext *format = NULL;
    if (avformat_open_input(&format, path, NULL, NULL) < 0) return sound;
    AVCodecContext *audio = NULL;
    SwrContext *resample = NULL;
    AVPacket *packet = av_packet_alloc();
    AVFrame *frame = av_frame_alloc();
    int stream = avformat_find_stream_info(format, NULL) < 0
                     ? -1
                     : av_find_best_stream(format, AVMEDIA_TYPE_AUDIO, -1, -1, NULL, 0);
    if (stream >= 0) audio = open_decoder(format, stream);
    AVChannelLayout stereo = AV_CHANNEL_LAYOUT_STEREO;
    if (audio && packet && frame &&
        swr_alloc_set_opts2(&resample, &stereo, AV_SAMPLE_FMT_FLT, OE_MOVIE_SOUND_RATE,
                            &audio->ch_layout, audio->sample_fmt, audio->sample_rate, 0,
                            NULL) >= 0 &&
        swr_init(resample) >= 0) {
        for (int ended = 0; !ended;) {
            if (av_read_frame(format, packet) < 0) {
                ended = 1;
                avcodec_send_packet(audio, NULL);
            } else {
                if (packet->stream_index == stream) avcodec_send_packet(audio, packet);
                av_packet_unref(packet);
            }
            while (avcodec_receive_frame(audio, frame) == 0) {
                int most = swr_get_out_samples(resample, frame->nb_samples);
                if (most <= 0 || !grow_sound(&sound, most)) continue;
                uint8_t *out =
                    (uint8_t *)(sound.samples + sound.frames * OE_MOVIE_SOUND_CHANNELS);
                int made = swr_convert(resample, &out, most,
                                       (const uint8_t **)frame->extended_data, frame->nb_samples);
                if (made > 0) sound.frames += (uint32_t)made;
            }
        }
    }
    swr_free(&resample);
    av_frame_free(&frame);
    av_packet_free(&packet);
    avcodec_free_context(&audio);
    avformat_close_input(&format);
    return sound;
}

OeMovie *oe_movie_open(const char *path, int32_t *width, int32_t *height, float *fps) {
    av_log_set_level(AV_LOG_QUIET);
    OeMovie *movie = calloc(1, sizeof(OeMovie));
    if (!movie) return NULL;
    if (avformat_open_input(&movie->format, path, NULL, NULL) < 0 ||
        avformat_find_stream_info(movie->format, NULL) < 0) {
        oe_movie_close(movie);
        return NULL;
    }
    movie->stream = av_find_best_stream(movie->format, AVMEDIA_TYPE_VIDEO, -1, -1, NULL, 0);
    if (movie->stream >= 0) movie->video = open_decoder(movie->format, movie->stream);
    movie->packet = av_packet_alloc();
    movie->frame = av_frame_alloc();
    if (!movie->video || !movie->packet || !movie->frame || movie->video->width <= 0 ||
        movie->video->height <= 0) {
        oe_movie_close(movie);
        return NULL;
    }
    AVRational rate =
        av_guess_frame_rate(movie->format, movie->format->streams[movie->stream], NULL);
    movie->width = movie->video->width;
    movie->height = movie->video->height;
    *width = movie->width;
    *height = movie->height;
    *fps = rate.num > 0 && rate.den > 0 ? (float)av_q2d(rate) : 15.0f;
    Sound sound = decode_sound(path);
    movie->sound = sound.samples;
    movie->sound_frames = sound.frames;
    return movie;
}

int32_t oe_movie_next_frame(OeMovie *movie, uint8_t *rgba) {
    for (;;) {
        int got = avcodec_receive_frame(movie->video, movie->frame);
        if (got == 0) {
            AVFrame *f = movie->frame;
            movie->scale = sws_getCachedContext(movie->scale, f->width, f->height, f->format,
                                                movie->width, movie->height, AV_PIX_FMT_RGBA,
                                                SWS_BILINEAR, NULL, NULL, NULL);
            if (!movie->scale) return 0;
            uint8_t *planes[4] = {rgba, NULL, NULL, NULL};
            int strides[4] = {movie->width * 4, 0, 0, 0};
            sws_scale(movie->scale, (const uint8_t *const *)f->data, f->linesize, 0, f->height,
                      planes, strides);
            return 1;
        }
        if (got != AVERROR(EAGAIN) || movie->draining) return 0;
        if (av_read_frame(movie->format, movie->packet) < 0) {
            movie->draining = 1;
            avcodec_send_packet(movie->video, NULL);
            continue;
        }
        if (movie->packet->stream_index == movie->stream) {
            avcodec_send_packet(movie->video, movie->packet);
        }
        av_packet_unref(movie->packet);
    }
}

float *oe_sound_decode(const char *path, uint32_t *frames) {
    av_log_set_level(AV_LOG_QUIET);
    Sound sound = decode_sound(path);
    if (!sound.frames) {
        free(sound.samples);
        sound.samples = NULL;
    }
    *frames = sound.frames;
    return sound.samples;
}

void oe_sound_free(float *samples) {
    free(samples);
}

const float *oe_movie_sound(const OeMovie *movie, uint32_t *frames) {
    *frames = movie->sound_frames;
    return movie->sound;
}

void oe_movie_close(OeMovie *movie) {
    if (!movie) return;
    sws_freeContext(movie->scale);
    av_frame_free(&movie->frame);
    av_packet_free(&movie->packet);
    avcodec_free_context(&movie->video);
    avformat_close_input(&movie->format);
    free(movie->sound);
    free(movie);
}
